#[cfg(not(target_os = "android"))]
use std::ffi::CString;
use std::fmt;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::ptr::NonNull;
use std::sync::{Arc, OnceLock};

#[cfg(target_os = "android")]
mod android;
pub mod hackrf_wrapper;

#[cfg(target_os = "android")]
pub use android::{set_android_activity_class, set_android_context};
pub use hackrf_wrapper::{
    BYTES_PER_BLOCK, HACKRF_OPERACAKE_ADDRESS_INVALID, HACKRF_OPERACAKE_MAX_BOARDS,
    HACKRF_OPERACAKE_MAX_DWELL_TIMES, HACKRF_OPERACAKE_MAX_FREQ_RANGES, MAX_SWEEP_RANGES,
    clkin_ctrl_signal, hackrf_bias_t_user_settting_req, hackrf_board_id, hackrf_board_rev,
    hackrf_bool_user_settting, hackrf_error, hackrf_operacake_dwell_time,
    hackrf_operacake_freq_range, hackrf_usb_board_id, operacake_ports, operacake_switching_mode,
    p1_ctrl_signal, p2_ctrl_signal, read_partid_serialno_t, rf_path_filter, sweep_style,
};

// --- Raw FFI helpers (C strings, enum mapping) -------------------------------

unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

fn board_id_from_raw(v: u8) -> hackrf_wrapper::hackrf_board_id {
    use hackrf_wrapper::hackrf_board_id::*;
    match v {
        0 => BOARD_ID_JELLYBEAN,
        1 => BOARD_ID_JAWBREAKER,
        2 => BOARD_ID_HACKRF1_OG,
        3 => BOARD_ID_RAD1O,
        4 => BOARD_ID_HACKRF1_R9,
        5 => BOARD_ID_PRALINE,
        0xFE => BOARD_ID_UNRECOGNIZED,
        _ => BOARD_ID_UNDETECTED,
    }
}

fn board_rev_from_raw(v: u8) -> hackrf_wrapper::hackrf_board_rev {
    use hackrf_wrapper::hackrf_board_rev::*;
    match v {
        0 => BOARD_REV_HACKRF1_OLD,
        1 => BOARD_REV_HACKRF1_R6,
        2 => BOARD_REV_HACKRF1_R7,
        3 => BOARD_REV_HACKRF1_R8,
        4 => BOARD_REV_HACKRF1_R9,
        5 => BOARD_REV_HACKRF1_R10,
        6 => BOARD_REV_PRALINE_R0_1,
        7 => BOARD_REV_PRALINE_R0_2,
        8 => BOARD_REV_PRALINE_R0_3,
        9 => BOARD_REV_PRALINE_R1_0,
        10 => BOARD_REV_PRALINE_R1_1,
        11 => BOARD_REV_PRALINE_R1_2,
        0x81 => BOARD_REV_GSG_HACKRF1_R6,
        0x82 => BOARD_REV_GSG_HACKRF1_R7,
        0x83 => BOARD_REV_GSG_HACKRF1_R8,
        0x84 => BOARD_REV_GSG_HACKRF1_R9,
        0x85 => BOARD_REV_GSG_HACKRF1_R10,
        0x86 => BOARD_REV_GSG_PRALINE_R0_1,
        0x87 => BOARD_REV_GSG_PRALINE_R0_2,
        0x88 => BOARD_REV_GSG_PRALINE_R0_3,
        0x89 => BOARD_REV_GSG_PRALINE_R1_0,
        0x8a => BOARD_REV_GSG_PRALINE_R1_1,
        0x8b => BOARD_REV_GSG_PRALINE_R1_2,
        0xFE => BOARD_REV_UNRECOGNIZED,
        _ => BOARD_REV_UNDETECTED,
    }
}

fn usb_board_id_from_raw(v: i32) -> hackrf_wrapper::hackrf_usb_board_id {
    use hackrf_wrapper::hackrf_usb_board_id::*;
    match v {
        0x604B => USB_BOARD_ID_JAWBREAKER,
        0x6089 => USB_BOARD_ID_HACKRF_ONE,
        0xCC15 => USB_BOARD_ID_RAD1O,
        _ => USB_BOARD_ID_INVALID,
    }
}

// --- Error handling -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Known(hackrf_wrapper::hackrf_error),
    Unknown(c_int),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Known(e) => write!(f, "{e} ({})", *e as c_int),
            Error::Unknown(code) => {
                write!(
                    f,
                    "{} ({code})",
                    hackrf_wrapper::hackrf_error::HACKRF_ERROR_OTHER
                )
            }
        }
    }
}

impl fmt::Display for hackrf_wrapper::hackrf_error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&unsafe { cstr_to_string(hackrf_wrapper::hackrf_error_name(*self)) })
    }
}

fn known_error(code: c_int) -> Option<hackrf_wrapper::hackrf_error> {
    use hackrf_wrapper::hackrf_error::*;
    Some(match code {
        0 => HACKRF_SUCCESS,
        1 => HACKRF_TRUE,
        -2 => HACKRF_ERROR_INVALID_PARAM,
        -5 => HACKRF_ERROR_NOT_FOUND,
        -6 => HACKRF_ERROR_BUSY,
        -11 => HACKRF_ERROR_NO_MEM,
        -1000 => HACKRF_ERROR_LIBUSB,
        -1001 => HACKRF_ERROR_THREAD,
        -1002 => HACKRF_ERROR_STREAMING_THREAD_ERR,
        -1003 => HACKRF_ERROR_STREAMING_STOPPED,
        -1004 => HACKRF_ERROR_STREAMING_EXIT_CALLED,
        -1005 => HACKRF_ERROR_USB_API_VERSION,
        -2000 => HACKRF_ERROR_NOT_LAST_DEVICE,
        -9999 => HACKRF_ERROR_OTHER,
        _ => return None,
    })
}

fn check(code: c_int) -> Result<(), Error> {
    if code == 0 {
        return Ok(());
    }
    Err(match known_error(code) {
        Some(e) => Error::Known(e),
        None => Error::Unknown(code),
    })
}

// --- FFI type ergonomics (Display, helpers on raw types) ---------------------

impl fmt::Display for hackrf_wrapper::hackrf_board_id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&unsafe { cstr_to_string(hackrf_wrapper::hackrf_board_id_name(*self)) })
    }
}

impl hackrf_wrapper::hackrf_board_id {
    /// Bitmask of the platforms (`HACKRF_PLATFORM_*`) this board belongs to.
    pub fn platform(self) -> u32 {
        unsafe { hackrf_wrapper::hackrf_board_id_platform(self) }
    }
}

impl fmt::Display for hackrf_wrapper::hackrf_board_rev {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&unsafe { cstr_to_string(hackrf_wrapper::hackrf_board_rev_name(*self)) })
    }
}

impl fmt::Display for hackrf_wrapper::hackrf_usb_board_id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&unsafe { cstr_to_string(hackrf_wrapper::hackrf_usb_board_id_name(*self)) })
    }
}

impl fmt::Display for hackrf_wrapper::rf_path_filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&unsafe { cstr_to_string(hackrf_wrapper::hackrf_filter_path_name(*self)) })
    }
}

impl hackrf_wrapper::read_partid_serialno_t {
    /// Part ID as a 16-digit hex string.
    pub fn part_id_hex(&self) -> String {
        format!("{:08x}{:08x}", self.part_id[0], self.part_id[1])
    }

    /// Serial number as a 32-digit hex string.
    pub fn serial_hex(&self) -> String {
        format!(
            "{:08x}{:08x}{:08x}{:08x}",
            self.serial_no[0], self.serial_no[1], self.serial_no[2], self.serial_no[3]
        )
    }
}

// --- Library-level free functions --------------------------------------------

/// libhackrf version string, e.g. `"2024.02.1"`.
pub fn library_version() -> String {
    unsafe { cstr_to_string(hackrf_wrapper::hackrf_library_version()) }
}

/// libhackrf release string.
pub fn library_release() -> String {
    unsafe { cstr_to_string(hackrf_wrapper::hackrf_library_release()) }
}

/// Nearest supported baseband filter bandwidth (Hz) to `bandwidth_hz`.
pub fn compute_baseband_filter_bw(bandwidth_hz: u32) -> u32 {
    unsafe { hackrf_wrapper::hackrf_compute_baseband_filter_bw(bandwidth_hz) }
}

/// Largest supported baseband filter bandwidth (Hz) strictly below `bandwidth_hz`.
pub fn compute_baseband_filter_bw_round_down_lt(bandwidth_hz: u32) -> u32 {
    unsafe { hackrf_wrapper::hackrf_compute_baseband_filter_bw_round_down_lt(bandwidth_hz) }
}

// --- Library init / exit -----------------------------------------------------

static INIT_CODE: OnceLock<c_int> = OnceLock::new();
static EXIT_CODE: OnceLock<c_int> = OnceLock::new();

fn ensure_init() -> Result<(), Error> {
    let code = *INIT_CODE.get_or_init(|| unsafe {
        #[cfg(target_os = "android")]
        {
            hackrf_wrapper::hackrf_init_on_android()
        }
        #[cfg(not(target_os = "android"))]
        {
            hackrf_wrapper::hackrf_init()
        }
    });
    check(code)
}

/// Calls `hackrf_exit`, exactly once for the life of the process.
///
/// Must only be called once every `HackrfDevice` *and* every stream started
/// from one has been dropped — a live [`RxStream`] or [`TxStream`] keeps its
/// device open on its own — per `hackrf_exit`'s contract; nothing calls this
/// automatically, so applications should call it themselves during shutdown.
pub fn ensure_exit() -> Result<(), Error> {
    let code = *EXIT_CODE.get_or_init(|| unsafe { hackrf_wrapper::hackrf_exit() });
    check(code)
}

// --- Device -------------------------------------------------------------------

/// The open `hackrf_device` handle itself, closed when the last holder goes
/// away. Kept behind an `Arc` so a running [`RxStream`] or [`TxStream`] keeps
/// the handle alive on its own: whatever order the values are dropped in,
/// `hackrf_close` runs once, after streaming has stopped.
struct DeviceInner {
    ptr: NonNull<hackrf_wrapper::hackrf_device>,
    /// On Android, the USB connection backing `ptr`'s file descriptor; closed on
    /// drop so the fd is released. `None` on desktop.
    #[cfg(target_os = "android")]
    connection: Option<android::Connection>,
}

// Opaque handle libhackrf hands to its own transfer thread, so it is not bound
// to the thread that opened it. Sharing it is limited to what the `&self`
// methods do — control transfers, which libusb serializes internally; starting
// and stopping streams on one device from several threads at once is still the
// caller's responsibility.
unsafe impl Send for DeviceInner {}
unsafe impl Sync for DeviceInner {}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        unsafe {
            hackrf_wrapper::hackrf_close(self.ptr.as_ptr());
        }
        // Release the USB fd on Android now that libhackrf is done with it.
        #[cfg(target_os = "android")]
        if let Some(connection) = self.connection.take() {
            android::close_connection(connection);
        }
    }
}

/// Thin wrapper around a `hackrf_device` handle.
///
/// Cloning yields another handle to the same device rather than opening a second
/// one; the device is closed once every handle, and every stream started from
/// one, has been dropped.
#[derive(Clone)]
pub struct HackrfDevice(Arc<DeviceInner>);

impl HackrfDevice {
    fn from_raw(
        ptr: *mut hackrf_wrapper::hackrf_device,
        #[cfg(target_os = "android")] connection: Option<android::Connection>,
    ) -> Self {
        Self(Arc::new(DeviceInner {
            ptr: NonNull::new(ptr)
                .expect("hackrf reported success but returned a null device pointer"),
            #[cfg(target_os = "android")]
            connection,
        }))
    }

    /// Open the first available HackRF.
    #[cfg(not(target_os = "android"))]
    pub fn open() -> Result<Self, Error> {
        ensure_init()?;
        let mut device: *mut hackrf_wrapper::hackrf_device = ptr::null_mut();
        let code = unsafe { hackrf_wrapper::hackrf_open(&mut device) };
        check(code)?;
        Ok(Self::from_raw(device))
    }

    #[cfg(target_os = "android")]
    pub fn open() -> Result<Self, Error> {
        ensure_init()?;
        let device = android::collect_devices(Some(1))?
            .into_iter()
            .next()
            .ok_or(Error::Known(
                hackrf_wrapper::hackrf_error::HACKRF_ERROR_NOT_FOUND,
            ))?;
        let connection = device.take_connection();
        Self::open_on_android(device.fd, connection)
    }

    #[cfg(not(target_os = "android"))]
    pub fn open_by_serial(serial: &str) -> Result<Self, Error> {
        ensure_init()?;
        let serial = CString::new(serial)
            .map_err(|_| Error::Known(hackrf_wrapper::hackrf_error::HACKRF_ERROR_INVALID_PARAM))?;
        let mut device: *mut hackrf_wrapper::hackrf_device = ptr::null_mut();
        let code = unsafe { hackrf_wrapper::hackrf_open_by_serial(serial.as_ptr(), &mut device) };
        check(code)?;
        Ok(Self::from_raw(device))
    }

    #[cfg(target_os = "android")]
    pub fn open_by_serial(serial: &str) -> Result<Self, Error> {
        ensure_init()?;
        let device = android::collect_devices(None)?
            .into_iter()
            .find(|d| d.serial == serial)
            .ok_or(Error::Known(
                hackrf_wrapper::hackrf_error::HACKRF_ERROR_NOT_FOUND,
            ))?;
        let connection = device.take_connection();
        Self::open_on_android(device.fd, connection)
    }

    #[cfg(target_os = "android")]
    fn open_on_android(fd: i32, connection: Option<android::Connection>) -> Result<Self, Error> {
        ensure_init()?;
        let mut device: *mut hackrf_wrapper::hackrf_device = ptr::null_mut();
        let code = unsafe { hackrf_wrapper::hackrf_open_on_android(fd, &mut device) };
        check(code)?;
        Ok(Self::from_raw(device, connection))
    }

    // --- Tuning & sample rate ---

    pub fn set_freq(&self, freq_hz: u64) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_freq(self.0.ptr.as_ptr(), freq_hz) })
    }

    pub fn set_freq_explicit(
        &self,
        if_freq_hz: u64,
        lo_freq_hz: u64,
        path: hackrf_wrapper::rf_path_filter,
    ) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_freq_explicit(
                self.0.ptr.as_ptr(),
                if_freq_hz,
                lo_freq_hz,
                path,
            )
        })
    }

    pub fn set_sample_rate(&self, freq_hz: f64) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_sample_rate(self.0.ptr.as_ptr(), freq_hz) })
    }

    pub fn set_sample_rate_manual(&self, freq_hz: u32, divider: u32) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_sample_rate_manual(self.0.ptr.as_ptr(), freq_hz, divider)
        })
    }

    pub fn set_baseband_filter_bandwidth(&self, bandwidth_hz: u32) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_baseband_filter_bandwidth(self.0.ptr.as_ptr(), bandwidth_hz)
        })
    }

    // --- Gain & amplifier ---

    pub fn set_lna_gain(&self, value: u32) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_lna_gain(self.0.ptr.as_ptr(), value) })
    }

    pub fn set_vga_gain(&self, value: u32) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_vga_gain(self.0.ptr.as_ptr(), value) })
    }

    /// TX (VGA/IF) output gain, 0-47dB in 1dB steps.
    pub fn set_txvga_gain(&self, value: u32) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_txvga_gain(self.0.ptr.as_ptr(), value) })
    }

    pub fn set_amp_enable(&self, enable: bool) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_amp_enable(self.0.ptr.as_ptr(), enable as u8) })
    }

    // --- Bias-tee (antenna port power) ---

    /// Bias-tee / antenna port power (3.3V, max 50mA).
    ///
    /// The firmware auto-disables this when returning to IDLE mode, so it
    /// cannot be latched on permanently; it must be re-enabled by the
    /// application whenever needed.
    pub fn set_antenna_enable(&self, enable: bool) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_antenna_enable(self.0.ptr.as_ptr(), enable as u8)
        })
    }

    pub fn set_user_bias_t_opts(
        &self,
        mut req: hackrf_wrapper::hackrf_bias_t_user_settting_req,
    ) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_user_bias_t_opts(self.0.ptr.as_ptr(), &mut req) })
    }

    // --- Clock & synchronization ---

    pub fn set_clkout_enable(&self, enable: bool) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_clkout_enable(self.0.ptr.as_ptr(), enable as u8)
        })
    }

    /// `true` if a valid external clock is present on the CLKIN port.
    pub fn clkin_status(&self) -> Result<bool, Error> {
        let mut status: u8 = 0;
        check(unsafe {
            hackrf_wrapper::hackrf_get_clkin_status(self.0.ptr.as_ptr(), &mut status)
        })?;
        Ok(status != 0)
    }

    pub fn set_hw_sync_mode(&self, enable: bool) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_hw_sync_mode(self.0.ptr.as_ptr(), enable as u8) })
    }

    // --- Device control (LEDs, UI, reset) ---

    /// Enable/disable the on-device UI (Rad1o, PortaPack, etc.).
    pub fn set_ui_enable(&self, enable: bool) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_ui_enable(self.0.ptr.as_ptr(), enable as u8) })
    }

    /// `state` is a bitfield: bit 0 is the first LED (USB on HackRF One),
    /// see `hackrf_set_leds` in `hackrf.h` for the full mapping.
    pub fn set_leds(&self, state: u8) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_leds(self.0.ptr.as_ptr(), state) })
    }

    pub fn reset(&self) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_reset(self.0.ptr.as_ptr()) })
    }

    // --- HackRF Pro ---

    /// Select signal for HackRF Pro SMA connector P1.
    pub fn set_p1_ctrl(&self, signal: hackrf_wrapper::p1_ctrl_signal) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_p1_ctrl(self.0.ptr.as_ptr(), signal) })
    }

    /// Select signal for HackRF Pro SMA connector P2.
    pub fn set_p2_ctrl(&self, signal: hackrf_wrapper::p2_ctrl_signal) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_p2_ctrl(self.0.ptr.as_ptr(), signal) })
    }

    /// Select clock input signal for HackRF Pro CLKIN.
    pub fn set_clkin_ctrl(&self, signal: hackrf_wrapper::clkin_ctrl_signal) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_clkin_ctrl(self.0.ptr.as_ptr(), signal) })
    }

    /// Enable/disable narrowband filter (HackRF Pro).
    pub fn set_narrowband_filter(&self, enable: bool) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_narrowband_filter(self.0.ptr.as_ptr(), enable as u8)
        })
    }

    /// Program the selected FPGA bitstream (HackRF Pro).
    pub fn set_fpga_bitstream(&self, index: u8) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_fpga_bitstream(self.0.ptr.as_ptr(), index) })
    }

    // --- Streaming ---

    /// Stop TX automatically after `value` samples go unfilled. 0 = no limit.
    pub fn set_tx_underrun_limit(&self, value: u32) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_tx_underrun_limit(self.0.ptr.as_ptr(), value) })
    }

    /// Stop RX automatically after `value` samples are dropped. 0 = no limit.
    pub fn set_rx_overrun_limit(&self, value: u32) -> Result<(), Error> {
        check(unsafe { hackrf_wrapper::hackrf_set_rx_overrun_limit(self.0.ptr.as_ptr(), value) })
    }

    /// Whether the device is currently streaming (RX, TX, or sweep).
    pub fn is_streaming(&self) -> Result<bool, Error> {
        is_streaming(&self.0)
    }

    /// Start receiving.
    ///
    /// `callback` runs on libhackrf's transfer thread and is handed each block
    /// of received bytes; return [`RxStreamResult::Stop`] to end the stream.
    /// Streaming continues until the returned [`RxStream`] is dropped (or the
    /// callback stops it), so keep the guard alive.
    pub fn start_rx<F>(&self, callback: F) -> Result<RxStream, Error>
    where
        F: FnMut(&[u8]) -> RxStreamResult + Send + 'static,
    {
        self.spawn_rx(hackrf_wrapper::hackrf_start_rx, callback)
    }

    /// Start a frequency sweep (configure it first with [`init_sweep`]).
    ///
    /// Like [`start_rx`], but each block is prefixed with a 10-byte frequency
    /// header (see `hackrf_init_sweep` in `hackrf.h`); the callback receives the
    /// raw bytes including that header.
    ///
    /// [`init_sweep`]: HackrfDevice::init_sweep
    /// [`start_rx`]: HackrfDevice::start_rx
    pub fn start_rx_sweep<F>(&self, callback: F) -> Result<RxStream, Error>
    where
        F: FnMut(&[u8]) -> RxStreamResult + Send + 'static,
    {
        self.spawn_rx(hackrf_wrapper::hackrf_start_rx_sweep, callback)
    }

    fn spawn_rx<F>(
        &self,
        start: unsafe extern "C" fn(
            *mut hackrf_wrapper::hackrf_device,
            hackrf_wrapper::hackrf_sample_block_cb_fn,
            *mut c_void,
        ) -> c_int,
        callback: F,
    ) -> Result<RxStream, Error>
    where
        F: FnMut(&[u8]) -> RxStreamResult + Send + 'static,
    {
        let ctx = Box::into_raw(Box::new(RxContext {
            callback: Box::new(callback),
        }));
        let code = unsafe {
            start(
                self.0.ptr.as_ptr(),
                Some(rx_trampoline),
                ctx.cast::<c_void>(),
            )
        };
        if let Err(e) = check(code) {
            drop(unsafe { Box::from_raw(ctx) });
            return Err(e);
        }
        Ok(RxStream {
            device: Arc::clone(&self.0),
            ctx,
        })
    }

    /// Start transmitting.
    ///
    /// `fill` runs on libhackrf's transfer thread; it fills the provided buffer
    /// with samples and returns a [`TxStreamResult`] describing how many bytes
    /// are valid and whether to continue. Keep the returned [`TxStream`] alive.
    ///
    /// `on_flush`, if given, fires once the last queued data has drained out of
    /// the device — the point at which the transmission has fully left it —
    /// receiving whether the flush succeeded.
    pub fn start_tx<F>(&self, fill: F, on_flush: Option<FlushCallback>) -> Result<TxStream, Error>
    where
        F: FnMut(&mut [u8]) -> TxStreamResult + Send + 'static,
    {
        let dev = self.0.ptr.as_ptr();
        let had_flush = on_flush.is_some();
        let ctx = Box::into_raw(Box::new(TxContext {
            callback: Box::new(fill),
            flush: on_flush,
        }));

        // The flush callback references `ctx`, so register it before starting.
        let outcome = (|| unsafe {
            if had_flush {
                check(hackrf_wrapper::hackrf_enable_tx_flush(
                    dev,
                    Some(tx_flush_trampoline),
                    ctx.cast::<c_void>(),
                ))?;
            }
            check(hackrf_wrapper::hackrf_start_tx(
                dev,
                Some(tx_trampoline),
                ctx.cast::<c_void>(),
            ))
        })();

        if let Err(e) = outcome {
            unsafe {
                // Roll back the flush callback we may have registered, then
                // reclaim the box.
                if had_flush {
                    hackrf_wrapper::hackrf_enable_tx_flush(dev, None, ptr::null_mut());
                }
                drop(Box::from_raw(ctx));
            }
            return Err(e);
        }

        Ok(TxStream {
            device: Arc::clone(&self.0),
            ctx,
            had_flush,
        })
    }

    /// Configure sweep mode over the given start/stop frequency ranges (in MHz).
    ///
    /// `bytes_per_step` is captured per tuning (a multiple of
    /// [`BYTES_PER_BLOCK`]); `offset` is added to each tuned frequency
    /// (`sample_rate / 2` is a good value).
    ///
    /// [`BYTES_PER_BLOCK`]: hackrf_wrapper::BYTES_PER_BLOCK
    pub fn init_sweep(
        &self,
        ranges: &[(u16, u16)],
        bytes_per_step: u32,
        step_width: u32,
        offset: u32,
        style: hackrf_wrapper::sweep_style,
    ) -> Result<(), Error> {
        if ranges.is_empty() || ranges.len() > hackrf_wrapper::MAX_SWEEP_RANGES {
            return Err(Error::Known(
                hackrf_wrapper::hackrf_error::HACKRF_ERROR_INVALID_PARAM,
            ));
        }
        let flat: Vec<u16> = ranges.iter().flat_map(|&(lo, hi)| [lo, hi]).collect();
        check(unsafe {
            hackrf_wrapper::hackrf_init_sweep(
                self.0.ptr.as_ptr(),
                flat.as_ptr(),
                ranges.len() as c_int,
                bytes_per_step,
                step_width,
                offset,
                style,
            )
        })
    }

    /// Size in bytes of a single USB transfer buffer.
    pub fn transfer_buffer_size(&self) -> usize {
        unsafe { hackrf_wrapper::hackrf_get_transfer_buffer_size(self.0.ptr.as_ptr()) }
    }

    /// Number of USB transfer buffers in the queue.
    pub fn transfer_queue_depth(&self) -> u32 {
        unsafe { hackrf_wrapper::hackrf_get_transfer_queue_depth(self.0.ptr.as_ptr()) }
    }

    // --- Board / device info ---

    /// Board type of the connected device.
    pub fn board_id(&self) -> Result<hackrf_wrapper::hackrf_board_id, Error> {
        let mut value: u8 = 0;
        check(unsafe { hackrf_wrapper::hackrf_board_id_read(self.0.ptr.as_ptr(), &mut value) })?;
        Ok(board_id_from_raw(value))
    }

    /// Hardware revision of the connected board.
    pub fn board_rev(&self) -> Result<hackrf_wrapper::hackrf_board_rev, Error> {
        let mut value: u8 = 0;
        check(unsafe { hackrf_wrapper::hackrf_board_rev_read(self.0.ptr.as_ptr(), &mut value) })?;
        Ok(board_rev_from_raw(value))
    }

    /// Firmware version string.
    pub fn version_string(&self) -> Result<String, Error> {
        let mut buf = [0 as c_char; 255];
        check(unsafe {
            hackrf_wrapper::hackrf_version_string_read(
                self.0.ptr.as_ptr(),
                buf.as_mut_ptr(),
                buf.len() as u8,
            )
        })?;
        Ok(unsafe { cstr_to_string(buf.as_ptr()) })
    }

    /// USB API version supported by the firmware, packed as `0xMMmm`
    /// (e.g. `0x0102` == 1.02).
    pub fn usb_api_version(&self) -> Result<u16, Error> {
        let mut version: u16 = 0;
        check(unsafe {
            hackrf_wrapper::hackrf_usb_api_version_read(self.0.ptr.as_ptr(), &mut version)
        })?;
        Ok(version)
    }

    /// Board part ID and serial number.
    pub fn part_id_serial_no(&self) -> Result<hackrf_wrapper::read_partid_serialno_t, Error> {
        let mut raw = hackrf_wrapper::read_partid_serialno_t {
            part_id: [0; 2],
            serial_no: [0; 4],
        };
        check(unsafe {
            hackrf_wrapper::hackrf_board_partid_serialno_read(self.0.ptr.as_ptr(), &mut raw)
        })?;
        Ok(raw)
    }

    /// Bitmask of platforms (`HACKRF_PLATFORM_*`) the firmware supports.
    pub fn supported_platforms(&self) -> Result<u32, Error> {
        let mut value: u32 = 0;
        check(unsafe {
            hackrf_wrapper::hackrf_supported_platform_read(self.0.ptr.as_ptr(), &mut value)
        })?;
        Ok(value)
    }

    // --- Opera Cake ---

    /// Addresses of connected Opera Cake add-on boards (invalid slots omitted).
    pub fn operacake_boards(&self) -> Result<Vec<u8>, Error> {
        let mut boards = [hackrf_wrapper::HACKRF_OPERACAKE_ADDRESS_INVALID;
            hackrf_wrapper::HACKRF_OPERACAKE_MAX_BOARDS];
        check(unsafe {
            hackrf_wrapper::hackrf_get_operacake_boards(self.0.ptr.as_ptr(), boards.as_mut_ptr())
        })?;
        Ok(boards
            .into_iter()
            .filter(|&b| b != hackrf_wrapper::HACKRF_OPERACAKE_ADDRESS_INVALID)
            .collect())
    }

    /// Set the port-switching mode of the Opera Cake board at `address`.
    pub fn set_operacake_mode(
        &self,
        address: u8,
        mode: hackrf_wrapper::operacake_switching_mode,
    ) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_operacake_mode(self.0.ptr.as_ptr(), address, mode)
        })
    }

    /// Current port-switching mode of the Opera Cake board at `address`.
    pub fn operacake_mode(
        &self,
        address: u8,
    ) -> Result<hackrf_wrapper::operacake_switching_mode, Error> {
        let mut mode = hackrf_wrapper::operacake_switching_mode::OPERACAKE_MODE_MANUAL;
        check(unsafe {
            hackrf_wrapper::hackrf_get_operacake_mode(self.0.ptr.as_ptr(), address, &mut mode)
        })?;
        Ok(mode)
    }

    /// Manually select the A and B ports on the Opera Cake board at `address`.
    pub fn set_operacake_ports(
        &self,
        address: u8,
        port_a: hackrf_wrapper::operacake_ports,
        port_b: hackrf_wrapper::operacake_ports,
    ) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_operacake_ports(
                self.0.ptr.as_ptr(),
                address,
                port_a as u8,
                port_b as u8,
            )
        })
    }

    /// Configure the dwell times used in `OPERACAKE_MODE_TIME`.
    pub fn set_operacake_dwell_times(
        &self,
        dwell_times: &[hackrf_wrapper::hackrf_operacake_dwell_time],
    ) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_operacake_dwell_times(
                self.0.ptr.as_ptr(),
                dwell_times.as_ptr() as *mut _,
                dwell_times.len() as u8,
            )
        })
    }

    /// Configure the frequency ranges used in `OPERACAKE_MODE_FREQUENCY`.
    pub fn set_operacake_freq_ranges(
        &self,
        freq_ranges: &[hackrf_wrapper::hackrf_operacake_freq_range],
    ) -> Result<(), Error> {
        check(unsafe {
            hackrf_wrapper::hackrf_set_operacake_freq_ranges(
                self.0.ptr.as_ptr(),
                freq_ranges.as_ptr() as *mut _,
                freq_ranges.len() as u8,
            )
        })
    }

    /// Run the Opera Cake GPIO self-test, returning the raw result bitfield.
    pub fn operacake_gpio_test(&self, address: u8) -> Result<u16, Error> {
        let mut result: u16 = 0;
        check(unsafe {
            hackrf_wrapper::hackrf_operacake_gpio_test(self.0.ptr.as_ptr(), address, &mut result)
        })?;
        Ok(result)
    }
}

// --- Streaming: callbacks, trampolines & guards ------------------------------

/// What an RX (or sweep) callback asks the driver to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxStreamResult {
    /// Keep streaming.
    Continue,
    /// Stop streaming after this block.
    Stop,
}

/// What a TX callback asks the driver to do next, carrying how many bytes it
/// wrote into the transmit buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStreamResult {
    /// Transmit `len` bytes and keep streaming.
    Continue(usize),
    /// Transmit `len` bytes, then stop streaming.
    Stop(usize),
}

impl TxStreamResult {
    /// Number of bytes written into the transmit buffer.
    pub fn len(self) -> usize {
        match self {
            Self::Continue(len) | Self::Stop(len) => len,
        }
    }
}

type RxCallback = Box<dyn FnMut(&[u8]) -> RxStreamResult + Send>;
type TxCallback = Box<dyn FnMut(&mut [u8]) -> TxStreamResult + Send>;

/// Notified once the last queued TX data has drained out of the device, with
/// whether the flush succeeded. Passed to [`HackrfDevice::start_tx`].
pub type FlushCallback = Box<dyn FnMut(bool) + Send>;

struct RxContext {
    callback: RxCallback,
}

struct TxContext {
    callback: TxCallback,
    flush: Option<FlushCallback>,
}

/// C trampoline for RX/sweep: recovers the Rust callback from `rx_ctx` and
/// feeds it the valid portion of the transfer buffer. Panics are caught (they
/// must not cross the FFI boundary) and turned into a stop.
unsafe extern "C" fn rx_trampoline(transfer: *mut hackrf_wrapper::hackrf_transfer) -> c_int {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let transfer = unsafe { &mut *transfer };
        let ctx = unsafe { &mut *(transfer.rx_ctx as *mut RxContext) };
        let len = transfer.valid_length.max(0) as usize;
        let buffer = unsafe { std::slice::from_raw_parts(transfer.buffer, len) };
        (ctx.callback)(buffer)
    }));
    match outcome {
        Ok(RxStreamResult::Continue) => 0,
        _ => 1,
    }
}

/// C trampoline for TX: lets the Rust callback fill the buffer, records how many
/// bytes are valid, and reports whether to continue. Panics stop the stream.
unsafe extern "C" fn tx_trampoline(transfer: *mut hackrf_wrapper::hackrf_transfer) -> c_int {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let transfer = unsafe { &mut *transfer };
        let ctx = unsafe { &mut *(transfer.tx_ctx as *mut TxContext) };
        let capacity = transfer.buffer_length.max(0) as usize;
        let buffer = unsafe { std::slice::from_raw_parts_mut(transfer.buffer, capacity) };
        let block = (ctx.callback)(buffer);
        transfer.valid_length = block.len().min(capacity) as c_int;
        block
    }));
    match outcome {
        Ok(TxStreamResult::Continue(_)) => 0,
        Ok(TxStreamResult::Stop(_)) => 1,
        Err(_) => {
            unsafe { (*transfer).valid_length = 0 };
            1
        }
    }
}

/// C trampoline for the TX flush callback. `flush_ctx` is the context pointer we
/// passed to `hackrf_enable_tx_flush`.
unsafe extern "C" fn tx_flush_trampoline(flush_ctx: *mut c_void, success: c_int) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if flush_ctx.is_null() {
            return;
        }
        let ctx = unsafe { &mut *(flush_ctx as *mut TxContext) };
        if let Some(callback) = ctx.flush.as_mut() {
            callback(success != 0);
        }
    }));
}

/// Whether the device is currently streaming (RX, TX, or sweep).
fn is_streaming(device: &DeviceInner) -> Result<bool, Error> {
    use hackrf_wrapper::hackrf_error::*;
    let code = unsafe { hackrf_wrapper::hackrf_is_streaming(device.ptr.as_ptr()) };
    if code == HACKRF_TRUE as c_int {
        Ok(true)
    } else if code == HACKRF_ERROR_STREAMING_THREAD_ERR as c_int
        || code == HACKRF_ERROR_STREAMING_STOPPED as c_int
        || code == HACKRF_ERROR_STREAMING_EXIT_CALLED as c_int
    {
        Ok(false)
    } else {
        Err(match known_error(code) {
            Some(e) => Error::Known(e),
            None => Error::Unknown(code),
        })
    }
}

/// Active RX (or sweep) stream. Streaming stops when this is dropped.
///
/// Holds a handle on the device of its own, so it keeps working even if the
/// [`HackrfDevice`] it was started from is dropped first.
#[must_use = "dropping the RxStream immediately stops streaming"]
pub struct RxStream {
    device: Arc<DeviceInner>,
    ctx: *mut RxContext,
}

// The context box is owned solely by the stream, and libhackrf only reaches it
// from the transfer thread, which is joined before the box is freed.
unsafe impl Send for RxStream {}

impl RxStream {
    /// Whether the underlying device is still streaming.
    pub fn is_streaming(&self) -> Result<bool, Error> {
        is_streaming(&self.device)
    }

    /// A handle to the device this stream runs on, for retuning or changing
    /// gain mid-stream.
    pub fn device(&self) -> HackrfDevice {
        HackrfDevice(Arc::clone(&self.device))
    }

    /// Stop streaming now (equivalent to dropping the stream).
    pub fn stop(self) {}
}

impl Drop for RxStream {
    fn drop(&mut self) {
        unsafe {
            // Stop first so the transfer thread is joined before the context
            // it dereferences is freed.
            hackrf_wrapper::hackrf_stop_rx(self.device.ptr.as_ptr());
            drop(Box::from_raw(self.ctx));
        }
    }
}

/// Active TX stream. Transmission stops when this is dropped.
///
/// Holds a handle on the device of its own, so it keeps working even if the
/// [`HackrfDevice`] it was started from is dropped first.
#[must_use = "dropping the TxStream immediately stops transmission"]
pub struct TxStream {
    device: Arc<DeviceInner>,
    ctx: *mut TxContext,
    had_flush: bool,
}

// Same ownership story as `RxStream`.
unsafe impl Send for TxStream {}

impl TxStream {
    /// Whether the underlying device is still streaming.
    pub fn is_streaming(&self) -> Result<bool, Error> {
        is_streaming(&self.device)
    }

    /// A handle to the device this stream runs on, for retuning or changing
    /// gain mid-stream.
    pub fn device(&self) -> HackrfDevice {
        HackrfDevice(Arc::clone(&self.device))
    }

    /// Stop transmitting now (equivalent to dropping the stream).
    pub fn stop(self) {}
}

impl Drop for TxStream {
    fn drop(&mut self) {
        unsafe {
            let dev = self.device.ptr.as_ptr();
            // Stop first so the transfer thread is joined before the context it
            // dereferences is freed.
            hackrf_wrapper::hackrf_stop_tx(dev);
            // Unregister the flush callback that referenced our context, so a
            // later stream can never invoke it against this freed box.
            if self.had_flush {
                hackrf_wrapper::hackrf_enable_tx_flush(dev, None, ptr::null_mut());
            }
            drop(Box::from_raw(self.ctx));
        }
    }
}

// --- Device enumeration -------------------------------------------------------

#[cfg(not(target_os = "android"))]
pub struct DeviceList {
    ptr: NonNull<hackrf_wrapper::hackrf_device_list_t>,
}

#[cfg(not(target_os = "android"))]
impl DeviceList {
    /// Enumerate all connected HackRF devices.
    pub fn new() -> Result<Self, Error> {
        ensure_init()?;
        let ptr = unsafe { hackrf_wrapper::hackrf_device_list() };
        let ptr = NonNull::new(ptr).ok_or(Error::Known(
            hackrf_wrapper::hackrf_error::HACKRF_ERROR_NO_MEM,
        ))?;
        Ok(Self { ptr })
    }

    /// Number of connected devices.
    pub fn len(&self) -> usize {
        unsafe { self.ptr.as_ref().devicecount.max(0) as usize }
    }

    /// `true` if no devices are connected.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Serial number of the device at `idx`, or `None` if out of range.
    pub fn serial_number(&self, idx: usize) -> Option<String> {
        if idx >= self.len() {
            return None;
        }
        unsafe {
            let arr = self.ptr.as_ref().serial_numbers;
            if arr.is_null() {
                return None;
            }
            let s = *arr.add(idx);
            Some(cstr_to_string(s))
        }
    }

    /// USB product identifier of the device at `idx`, or `None` if out of range.
    pub fn usb_board_id(&self, idx: usize) -> Option<hackrf_wrapper::hackrf_usb_board_id> {
        if idx >= self.len() {
            return None;
        }
        unsafe {
            let arr = self.ptr.as_ref().usb_board_ids;
            if arr.is_null() {
                return None;
            }
            // Read as the underlying i32 and map, to avoid constructing an
            // invalid enum value if the firmware reports an unknown product id.
            let raw = *(arr as *const c_int).add(idx);
            Some(usb_board_id_from_raw(raw))
        }
    }

    /// Whether the device at `idx` shares its USB bus with another HackRF.
    pub fn bus_sharing(&self, idx: usize) -> Option<bool> {
        if idx >= self.len() {
            return None;
        }
        let r = unsafe {
            hackrf_wrapper::hackrf_device_list_bus_sharing(self.ptr.as_ptr(), idx as c_int)
        };
        Some(r != 0)
    }

    /// Open the device at `idx`.
    pub fn open(&self, idx: usize) -> Result<HackrfDevice, Error> {
        let mut device: *mut hackrf_wrapper::hackrf_device = ptr::null_mut();
        let code = unsafe {
            hackrf_wrapper::hackrf_device_list_open(self.ptr.as_ptr(), idx as c_int, &mut device)
        };
        check(code)?;
        Ok(HackrfDevice::from_raw(device))
    }
}

#[cfg(not(target_os = "android"))]
impl Drop for DeviceList {
    fn drop(&mut self) {
        unsafe {
            hackrf_wrapper::hackrf_device_list_free(self.ptr.as_ptr());
        }
    }
}

#[cfg(target_os = "android")]
pub struct DeviceList {
    devices: Vec<android::AndroidDevice>,
}

#[cfg(target_os = "android")]
impl DeviceList {
    /// Enumerate all connected HackRF devices (may prompt for USB permission).
    pub fn new() -> Result<Self, Error> {
        ensure_init()?;
        Ok(Self {
            devices: android::collect_devices(None)?,
        })
    }

    /// Number of connected devices.
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// `true` if no devices are connected.
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// Serial number of the device at `idx`, or `None` if out of range.
    pub fn serial_number(&self, idx: usize) -> Option<String> {
        self.devices.get(idx).map(|d| d.serial.clone())
    }

    /// USB product identifier of the device at `idx`, or `None` if out of range.
    pub fn usb_board_id(&self, idx: usize) -> Option<hackrf_wrapper::hackrf_usb_board_id> {
        self.devices
            .get(idx)
            .map(|d| usb_board_id_from_raw(d.pid as c_int))
    }

    /// Not available on Android (the platform exposes file descriptors, not USB
    /// bus topology); always returns `None`.
    pub fn bus_sharing(&self, _idx: usize) -> Option<bool> {
        None
    }

    /// Open the device at `idx`. Returns `HACKRF_ERROR_BUSY` if that entry was
    /// already opened (its USB connection has been transferred to a device).
    pub fn open(&self, idx: usize) -> Result<HackrfDevice, Error> {
        let device = self.devices.get(idx).ok_or(Error::Known(
            hackrf_wrapper::hackrf_error::HACKRF_ERROR_INVALID_PARAM,
        ))?;
        let connection = device.take_connection().ok_or(Error::Known(
            hackrf_wrapper::hackrf_error::HACKRF_ERROR_BUSY,
        ))?;
        HackrfDevice::open_on_android(device.fd, Some(connection))
    }
}
