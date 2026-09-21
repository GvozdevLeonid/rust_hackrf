//! Android USB glue for libhackrf.
//!
//! On Android libusb cannot enumerate or open USB devices directly — access is
//! mediated by the Java `UsbManager`, which requires a per-device permission
//! grant and yields an open file descriptor. This module drives that flow over
//! JNI and hands the resulting file descriptors to the Android-patched
//! `hackrf_open_on_android`.
//!
//! The runtime `JavaVM` is captured in [`JNI_OnLoad`] (so load this library via
//! `System.loadLibrary` once at startup), and the `Context` is discovered from
//! the configured activity class' static `mActivity` field (python-for-android's
//! by default; see [`set_android_activity_class`]) or supplied directly via
//! [`set_android_context`]. Both are published into `ndk_context` on first use.

use std::cell::Cell;
use std::os::raw::c_void;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::sys::{JNI_VERSION_1_6, jint};
use jni::{JNIEnv, JavaVM};

use super::Error;
use super::hackrf_wrapper::{hackrf_error, hackrf_usb_board_id};

/// Great Scott Gadgets USB vendor ID (shared by all HackRF variants).
const HACKRF_USB_VID: i32 = 0x1d50;

/// Valid HackRF USB product IDs — exactly the recognized `hackrf_usb_board_id`
/// values, kept in sync with the FFI enum rather than duplicated as literals.
const HACKRF_USB_PIDS: [i32; 3] = [
    hackrf_usb_board_id::USB_BOARD_ID_JAWBREAKER as i32,
    hackrf_usb_board_id::USB_BOARD_ID_HACKRF_ONE as i32,
    hackrf_usb_board_id::USB_BOARD_ID_RAD1O as i32,
];
const PERMISSION_ACTION: &str = "libusb.android.USB_PERMISSION";
const PERMISSION_POLL_INTERVAL_MS: u64 = 100;
const PERMISSION_TIMEOUT_MS: u64 = 10_000;

/// An owned `UsbDeviceConnection` reference. libusb wraps the file descriptor
/// but does not own it, so this must stay alive while the fd is in use; its
/// `close()` releases the fd.
pub(super) type Connection = GlobalRef;

/// A HackRF discovered through the Android `UsbManager`.
///
/// Owns the open `UsbDeviceConnection` that yielded its file descriptor. When
/// the device is opened through libhackrf, ownership of the connection is
/// transferred to the `HackrfDevice` via [`take_connection`]; otherwise the
/// connection is closed when this is dropped.
///
/// [`take_connection`]: AndroidDevice::take_connection
pub(super) struct AndroidDevice {
    pub(super) fd: i32,
    pub(super) pid: u16,
    pub(super) serial: String,
    connection: Cell<Option<Connection>>,
}

impl AndroidDevice {
    /// Take ownership of the USB connection, leaving the device without one so
    /// its `Drop` won't close the (now transferred) file descriptor.
    pub(super) fn take_connection(&self) -> Option<Connection> {
        self.connection.take()
    }
}

impl Drop for AndroidDevice {
    fn drop(&mut self) {
        // Only reached for devices that were never opened (connection not
        // taken): release the USB fd instead of leaking it.
        if let Some(connection) = self.connection.take() {
            close_connection(connection);
        }
    }
}

/// The process `JavaVM`, captured in [`JNI_OnLoad`].
static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// The Android `Context` (`PythonActivity.mActivity`), kept alive for the
/// lifetime of the process once discovered.
static ANDROID_CONTEXT: OnceLock<GlobalRef> = OnceLock::new();

fn other() -> Error {
    Error::Known(hackrf_error::HACKRF_ERROR_OTHER)
}

/// JNI name of the activity class whose static `mActivity` field holds the
/// `Context`. Overridable via [`set_android_activity_class`].
static ACTIVITY_CLASS: OnceLock<String> = OnceLock::new();

/// python-for-android's default activity, used when nothing else is configured.
const DEFAULT_ACTIVITY_CLASS: &str = "org/kivy/android/PythonActivity";

fn activity_class() -> &'static str {
    ACTIVITY_CLASS
        .get()
        .map(String::as_str)
        .unwrap_or(DEFAULT_ACTIVITY_CLASS)
}

/// Override the activity class used to auto-discover the Android `Context`.
///
/// `class_path` is the JNI (slash-separated) form, e.g.
/// `"com/example/MainActivity"`; the class must expose a
/// `public static <ClassName> mActivity` field. Call once at startup, before
/// the first device call. Unnecessary if you use [`set_android_context`] or the
/// default python-for-android activity.
pub fn set_android_activity_class(class_path: &str) {
    let _ = ACTIVITY_CLASS.set(class_path.to_owned());
}

/// Provide the Android `Context` directly, bypassing activity discovery.
///
/// `context` is a raw JNI `jobject` for any `android.content.Context` (Activity,
/// Service, or Application). Use this for hosts that already hold the `Context`
/// or use a non-standard activity layout. Call once at startup.
///
/// # Safety
/// `context` must be a valid reference to a `Context`, on a thread attached to
/// the `JavaVM` captured in [`JNI_OnLoad`].
pub unsafe fn set_android_context(context: *mut c_void) -> Result<(), Error> {
    let vm = JAVA_VM.get().ok_or_else(other)?;
    let env = vm.attach_current_thread().map_err(|_| other())?;
    let obj = unsafe { JObject::from_raw(context.cast()) };
    let global = env.new_global_ref(&obj).map_err(|_| other())?;
    publish_context(vm, global);
    Ok(())
}

/// Publishes the `Context` into `ndk_context`, exactly once per process
/// (initializing it twice would panic).
fn publish_context(vm: &JavaVM, context: GlobalRef) {
    if ANDROID_CONTEXT.set(context).is_ok() {
        let stored = ANDROID_CONTEXT.get().unwrap();
        unsafe {
            ndk_context::initialize_android_context(
                vm.get_java_vm_pointer().cast::<c_void>(),
                stored.as_obj().as_raw().cast::<c_void>(),
            );
        }
    }
}

/// Captures the `JavaVM` when the shared library is loaded by the JVM.
///
/// Only runs if the library is loaded through `System.loadLibrary`; ensure the
/// host does so once at startup.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: *mut jni::sys::JavaVM, _reserved: *mut c_void) -> jint {
    if let Ok(vm) = unsafe { JavaVM::from_raw(vm) } {
        let _ = JAVA_VM.set(vm);
    }
    JNI_VERSION_1_6
}

/// Ensures `ndk_context` is populated with the `JavaVM` and `Context`.
///
/// Idempotent. Unless a `Context` was already supplied via
/// [`set_android_context`], it is discovered from the configured activity
/// class' static `mActivity` field (see [`set_android_activity_class`]) and
/// cached for the process lifetime.
fn ensure_android_context() -> Result<(), Error> {
    if ANDROID_CONTEXT.get().is_some() {
        return Ok(());
    }

    let vm = JAVA_VM.get().ok_or_else(other)?;
    let mut env = vm.attach_current_thread().map_err(|_| other())?;

    let class = activity_class();
    let class_obj = env.find_class(class).map_err(|_| other())?;
    let signature = format!("L{class};");
    let activity = env
        .get_static_field(class_obj, "mActivity", signature.as_str())
        .and_then(|v| v.l())
        .map_err(|_| other())?;
    if activity.is_null() {
        return Err(other());
    }
    let global = env.new_global_ref(&activity).map_err(|_| other())?;
    publish_context(vm, global);
    Ok(())
}

/// Enumerate connected HackRFs, prompting for USB permission where needed.
///
/// `limit` caps how many devices to collect (`open` passes `Some(1)`). Blocks
/// while waiting for permission grants, so must not run on the UI thread.
pub(super) fn collect_devices(limit: Option<usize>) -> Result<Vec<AndroidDevice>, Error> {
    ensure_android_context()?;

    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|_| other())?;
    let mut env = vm.attach_current_thread().map_err(|_| other())?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };

    let manager = usb_manager(&mut env, &context)?;
    if manager.is_null() {
        // The device has no USB host support: no HackRF can be attached.
        return Ok(Vec::new());
    }
    let pending = build_permission_intent(&mut env, &context)?;

    let mut ready: Vec<AndroidDevice> = Vec::new();
    let mut awaiting: Vec<GlobalRef> = Vec::new();

    let map = env
        .call_method(&manager, "getDeviceList", "()Ljava/util/HashMap;", &[])
        .and_then(|v| v.l())
        .map_err(|_| other())?;
    let values = env
        .call_method(&map, "values", "()Ljava/util/Collection;", &[])
        .and_then(|v| v.l())
        .map_err(|_| other())?;
    let iter = env
        .call_method(&values, "iterator", "()Ljava/util/Iterator;", &[])
        .and_then(|v| v.l())
        .map_err(|_| other())?;

    loop {
        let has_next = env
            .call_method(&iter, "hasNext", "()Z", &[])
            .and_then(|v| v.z())
            .map_err(|_| other())?;
        if !has_next {
            break;
        }
        let device = env
            .call_method(&iter, "next", "()Ljava/lang/Object;", &[])
            .and_then(|v| v.l())
            .map_err(|_| other())?;

        if !is_hackrf(&mut env, &device)? {
            continue;
        }

        if has_permission(&mut env, &manager, &device)? {
            if let Some(dev) = open_device(&mut env, &manager, &device)? {
                ready.push(dev);
            }
        } else {
            request_permission(&mut env, &manager, &device, &pending)?;
            awaiting.push(env.new_global_ref(&device).map_err(|_| other())?);
        }

        if let Some(limit) = limit
            && ready.len() + awaiting.len() >= limit
        {
            break;
        }
    }

    // Wait for the devices that needed a permission prompt, then open them.
    for granted_device in awaiting {
        let device = granted_device.as_obj();
        if poll_permission(&mut env, &manager, device)?
            && let Some(dev) = open_device(&mut env, &manager, device)?
        {
            ready.push(dev);
        }
    }

    Ok(ready)
}

/// `context.getSystemService(Context.USB_SERVICE)`.
fn usb_manager<'local>(
    env: &mut JNIEnv<'local>,
    context: &JObject,
) -> Result<JObject<'local>, Error> {
    let name = env.new_string("usb").map_err(|_| other())?;
    env.call_method(
        context,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&name)],
    )
    .and_then(|v| v.l())
    .map_err(|_| other())
}

/// Builds the (mutable) `PendingIntent` handed to `requestPermission`.
fn build_permission_intent<'local>(
    env: &mut JNIEnv<'local>,
    context: &JObject,
) -> Result<JObject<'local>, Error> {
    let action = env.new_string(PERMISSION_ACTION).map_err(|_| other())?;
    let intent = env
        .new_object(
            "android/content/Intent",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|_| other())?;
    // FLAG_MUTABLE — required on API 31+ so the system can fill in the result.
    const FLAG_MUTABLE: jint = 0x0200_0000;
    env.call_static_method(
        "android/app/PendingIntent",
        "getBroadcast",
        "(Landroid/content/Context;ILandroid/content/Intent;I)Landroid/app/PendingIntent;",
        &[
            JValue::Object(context),
            JValue::Int(0),
            JValue::Object(&intent),
            JValue::Int(FLAG_MUTABLE),
        ],
    )
    .and_then(|v| v.l())
    .map_err(|_| other())
}

/// Whether `device`'s VID/PID identify it as a HackRF.
fn is_hackrf(env: &mut JNIEnv, device: &JObject) -> Result<bool, Error> {
    let vid = env
        .call_method(device, "getVendorId", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|_| other())?;
    if vid != HACKRF_USB_VID {
        return Ok(false);
    }
    let pid = env
        .call_method(device, "getProductId", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|_| other())?;
    Ok(HACKRF_USB_PIDS.contains(&pid))
}

fn has_permission(env: &mut JNIEnv, manager: &JObject, device: &JObject) -> Result<bool, Error> {
    env.call_method(
        manager,
        "hasPermission",
        "(Landroid/hardware/usb/UsbDevice;)Z",
        &[JValue::Object(device)],
    )
    .and_then(|v| v.z())
    .map_err(|_| other())
}

fn request_permission(
    env: &mut JNIEnv,
    manager: &JObject,
    device: &JObject,
    pending: &JObject,
) -> Result<(), Error> {
    env.call_method(
        manager,
        "requestPermission",
        "(Landroid/hardware/usb/UsbDevice;Landroid/app/PendingIntent;)V",
        &[JValue::Object(device), JValue::Object(pending)],
    )
    .map(|_| ())
    .map_err(|_| other())
}

/// Polls `hasPermission` until granted or the timeout elapses.
fn poll_permission(env: &mut JNIEnv, manager: &JObject, device: &JObject) -> Result<bool, Error> {
    let attempts = PERMISSION_TIMEOUT_MS / PERMISSION_POLL_INTERVAL_MS;
    for _ in 0..attempts {
        if has_permission(env, manager, device)? {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(PERMISSION_POLL_INTERVAL_MS));
    }
    Ok(false)
}

/// Opens `device` and returns its file descriptor plus identifying info.
///
/// Returns `Ok(None)` if the device could not be opened. The returned
/// [`AndroidDevice`] owns the `UsbDeviceConnection`, keeping the descriptor
/// valid until the device is opened through libhackrf or dropped.
fn open_device(
    env: &mut JNIEnv,
    manager: &JObject,
    device: &JObject,
) -> Result<Option<AndroidDevice>, Error> {
    let connection = env
        .call_method(
            manager,
            "openDevice",
            "(Landroid/hardware/usb/UsbDevice;)Landroid/hardware/usb/UsbDeviceConnection;",
            &[JValue::Object(device)],
        )
        .and_then(|v| v.l())
        .map_err(|_| other())?;
    if connection.is_null() {
        return Ok(None);
    }

    let fd = env
        .call_method(&connection, "getFileDescriptor", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|_| other())?;
    if fd < 0 {
        return Ok(None);
    }

    let pid = env
        .call_method(device, "getProductId", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|_| other())? as u16;
    let serial = serial_number(env, device);

    // Own the connection so the borrowed fd stays valid; it then travels with
    // the device — into the HackrfDevice when opened, or closed on drop.
    let connection = env.new_global_ref(&connection).map_err(|_| other())?;

    Ok(Some(AndroidDevice {
        fd,
        pid,
        serial,
        connection: Cell::new(Some(connection)),
    }))
}

/// Closes a `UsbDeviceConnection`, releasing its file descriptor. Best-effort:
/// errors during teardown are ignored.
pub(super) fn close_connection(connection: Connection) {
    if let Some(vm) = JAVA_VM.get()
        && let Ok(mut env) = vm.attach_current_thread()
    {
        let _ = env.call_method(connection.as_obj(), "close", "()V", &[]);
    }
}

/// Reads `device.getSerialNumber()`, or an empty string if unavailable.
///
/// `getSerialNumber` can throw (e.g. `SecurityException`); the serial is
/// non-essential, so any error is swallowed — clearing the pending exception so
/// it does not poison later JNI calls — and an empty string returned.
fn serial_number(env: &mut JNIEnv, device: &JObject) -> String {
    let value = match env
        .call_method(device, "getSerialNumber", "()Ljava/lang/String;", &[])
        .and_then(|v| v.l())
    {
        Ok(value) => value,
        Err(_) => {
            let _ = env.exception_clear();
            return String::new();
        }
    };
    if value.is_null() {
        return String::new();
    }
    match env.get_string(&JString::from(value)) {
        Ok(serial) => serial.into(),
        Err(_) => {
            let _ = env.exception_clear();
            String::new()
        }
    }
}
