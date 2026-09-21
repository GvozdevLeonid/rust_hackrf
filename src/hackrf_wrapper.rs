#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

pub const SAMPLES_PER_BLOCK: usize = 8192;
pub const BYTES_PER_BLOCK: usize = 16384;
pub const MAX_SWEEP_RANGES: usize = 10;

pub const HACKRF_OPERACAKE_ADDRESS_INVALID: u8 = 0xFF;
pub const HACKRF_OPERACAKE_MAX_BOARDS: usize = 8;
pub const HACKRF_OPERACAKE_MAX_DWELL_TIMES: usize = 16;
pub const HACKRF_OPERACAKE_MAX_FREQ_RANGES: usize = 8;

pub const HACKRF_BOARD_REV_GSG: u8 = 0x80;

pub const HACKRF_PLATFORM_JAWBREAKER: u32 = 1 << 0;
pub const HACKRF_PLATFORM_HACKRF1_OG: u32 = 1 << 1;
pub const HACKRF_PLATFORM_RAD1O: u32 = 1 << 2;
pub const HACKRF_PLATFORM_HACKRF1_R9: u32 = 1 << 3;
pub const HACKRF_PLATFORM_PRALINE: u32 = 1 << 4;

// ---------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum hackrf_error {
    HACKRF_SUCCESS = 0,
    HACKRF_TRUE = 1,
    HACKRF_ERROR_INVALID_PARAM = -2,
    HACKRF_ERROR_NOT_FOUND = -5,
    HACKRF_ERROR_BUSY = -6,
    HACKRF_ERROR_NO_MEM = -11,
    HACKRF_ERROR_LIBUSB = -1000,
    HACKRF_ERROR_THREAD = -1001,
    HACKRF_ERROR_STREAMING_THREAD_ERR = -1002,
    HACKRF_ERROR_STREAMING_STOPPED = -1003,
    HACKRF_ERROR_STREAMING_EXIT_CALLED = -1004,
    HACKRF_ERROR_USB_API_VERSION = -1005,
    HACKRF_ERROR_NOT_LAST_DEVICE = -2000,
    HACKRF_ERROR_OTHER = -9999,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum hackrf_board_id {
    BOARD_ID_JELLYBEAN = 0,
    BOARD_ID_JAWBREAKER = 1,
    BOARD_ID_HACKRF1_OG = 2,
    BOARD_ID_RAD1O = 3,
    BOARD_ID_HACKRF1_R9 = 4,
    BOARD_ID_PRALINE = 5,
    BOARD_ID_UNRECOGNIZED = 0xFE,
    BOARD_ID_UNDETECTED = 0xFF,
}

/// Deprecated alias, provided for API compatibility.
pub const BOARD_ID_HACKRF_ONE: hackrf_board_id = hackrf_board_id::BOARD_ID_HACKRF1_OG;
/// Deprecated alias, provided for API compatibility.
pub const BOARD_ID_INVALID: hackrf_board_id = hackrf_board_id::BOARD_ID_UNDETECTED;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum hackrf_board_rev {
    BOARD_REV_HACKRF1_OLD = 0,
    BOARD_REV_HACKRF1_R6 = 1,
    BOARD_REV_HACKRF1_R7 = 2,
    BOARD_REV_HACKRF1_R8 = 3,
    BOARD_REV_HACKRF1_R9 = 4,
    BOARD_REV_HACKRF1_R10 = 5,

    BOARD_REV_PRALINE_R0_1 = 6,
    BOARD_REV_PRALINE_R0_2 = 7,
    BOARD_REV_PRALINE_R0_3 = 8,
    BOARD_REV_PRALINE_R1_0 = 9,
    BOARD_REV_PRALINE_R1_1 = 10,
    BOARD_REV_PRALINE_R1_2 = 11,

    BOARD_REV_GSG_HACKRF1_R6 = 0x81,
    BOARD_REV_GSG_HACKRF1_R7 = 0x82,
    BOARD_REV_GSG_HACKRF1_R8 = 0x83,
    BOARD_REV_GSG_HACKRF1_R9 = 0x84,
    BOARD_REV_GSG_HACKRF1_R10 = 0x85,

    BOARD_REV_GSG_PRALINE_R0_1 = 0x86,
    BOARD_REV_GSG_PRALINE_R0_2 = 0x87,
    BOARD_REV_GSG_PRALINE_R0_3 = 0x88,
    BOARD_REV_GSG_PRALINE_R1_0 = 0x89,
    BOARD_REV_GSG_PRALINE_R1_1 = 0x8a,
    BOARD_REV_GSG_PRALINE_R1_2 = 0x8b,

    BOARD_REV_UNRECOGNIZED = 0xFE,
    BOARD_REV_UNDETECTED = 0xFF,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum hackrf_usb_board_id {
    USB_BOARD_ID_JAWBREAKER = 0x604B,
    USB_BOARD_ID_HACKRF_ONE = 0x6089,
    USB_BOARD_ID_RAD1O = 0xCC15,
    USB_BOARD_ID_INVALID = 0xFFFF,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum rf_path_filter {
    RF_PATH_FILTER_BYPASS = 0,
    RF_PATH_FILTER_LOW_PASS = 1,
    RF_PATH_FILTER_HIGH_PASS = 2,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum operacake_ports {
    OPERACAKE_PA1 = 0,
    OPERACAKE_PA2 = 1,
    OPERACAKE_PA3 = 2,
    OPERACAKE_PA4 = 3,
    OPERACAKE_PB1 = 4,
    OPERACAKE_PB2 = 5,
    OPERACAKE_PB3 = 6,
    OPERACAKE_PB4 = 7,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum operacake_switching_mode {
    OPERACAKE_MODE_MANUAL = 0,
    OPERACAKE_MODE_FREQUENCY = 1,
    OPERACAKE_MODE_TIME = 2,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum sweep_style {
    LINEAR = 0,
    INTERLEAVED = 1,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum p1_ctrl_signal {
    P1_SIGNAL_TRIGGER_IN = 0,
    P1_SIGNAL_AUX_CLK1 = 1,
    P1_SIGNAL_CLKIN = 2,
    P1_SIGNAL_TRIGGER_OUT = 3,
    P1_SIGNAL_P22_CLKIN = 4,
    P1_SIGNAL_P2_5 = 5,
    P1_SIGNAL_NC = 6,
    P1_SIGNAL_AUX_CLK2 = 7,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum p2_ctrl_signal {
    P2_SIGNAL_CLK3 = 0,
    P2_SIGNAL_TRIGGER_IN = 2,
    P2_SIGNAL_TRIGGER_OUT = 3,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum clkin_ctrl_signal {
    CLKIN_SIGNAL_P1 = 0,
    CLKIN_SIGNAL_P22 = 1,
}

// ---------------------------------------------------------------------
// Structs & typedefs
// ---------------------------------------------------------------------
pub enum hackrf_device {}

#[repr(C)]
pub struct hackrf_transfer {
    pub device: *mut hackrf_device,
    pub buffer: *mut u8,
    pub buffer_length: c_int,
    pub valid_length: c_int,
    pub rx_ctx: *mut c_void,
    pub tx_ctx: *mut c_void,
}

#[repr(C)]
pub struct read_partid_serialno_t {
    pub part_id: [u32; 2],
    pub serial_no: [u32; 4],
}

#[repr(C)]
pub struct hackrf_operacake_dwell_time {
    pub dwell: u32,
    pub port: u8,
}

#[repr(C)]
pub struct hackrf_operacake_freq_range {
    pub freq_min: u16,
    pub freq_max: u16,
    pub port: u8,
}

#[repr(C)]
pub struct hackrf_bool_user_settting {
    pub do_update: bool,
    pub change_on_mode_entry: bool,
    pub enabled: bool,
}

#[repr(C)]
pub struct hackrf_bias_t_user_settting_req {
    pub tx: hackrf_bool_user_settting,
    pub rx: hackrf_bool_user_settting,
    pub off: hackrf_bool_user_settting,
}

#[repr(C)]
pub struct hackrf_device_list {
    pub serial_numbers: *mut *mut c_char,
    pub usb_board_ids: *mut hackrf_usb_board_id,
    pub usb_device_index: *mut c_int,
    pub devicecount: c_int,
    pub usb_devices: *mut *mut c_void,
    pub usb_devicecount: c_int,
}

pub type hackrf_device_list_t = hackrf_device_list;

// ---------------------------------------------------------------------
// Callback function pointer types
// ---------------------------------------------------------------------

pub type hackrf_sample_block_cb_fn =
    Option<unsafe extern "C" fn(transfer: *mut hackrf_transfer) -> c_int>;

pub type hackrf_flush_cb_fn = Option<unsafe extern "C" fn(flush_ctx: *mut c_void, success: c_int)>;

// ---------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------

// `-lhackrf` and its search path are emitted by build.rs.
unsafe extern "C" {
    // Library init / info
    pub fn hackrf_init() -> c_int;
    pub fn hackrf_exit() -> c_int;
    pub fn hackrf_library_version() -> *const c_char;
    pub fn hackrf_library_release() -> *const c_char;

    // Device listing, opening, closing
    pub fn hackrf_device_list() -> *mut hackrf_device_list_t;
    pub fn hackrf_device_list_open(
        list: *mut hackrf_device_list_t,
        idx: c_int,
        device: *mut *mut hackrf_device,
    ) -> c_int;
    pub fn hackrf_device_list_bus_sharing(list: *mut hackrf_device_list_t, idx: c_int) -> c_int;
    pub fn hackrf_device_list_free(list: *mut hackrf_device_list_t);
    pub fn hackrf_open(device: *mut *mut hackrf_device) -> c_int;
    pub fn hackrf_open_by_serial(
        desired_serial_number: *const c_char,
        device: *mut *mut hackrf_device,
    ) -> c_int;
    pub fn hackrf_close(device: *mut hackrf_device) -> c_int;

    // Streaming
    pub fn hackrf_start_rx(
        device: *mut hackrf_device,
        callback: hackrf_sample_block_cb_fn,
        rx_ctx: *mut c_void,
    ) -> c_int;
    pub fn hackrf_stop_rx(device: *mut hackrf_device) -> c_int;
    pub fn hackrf_start_tx(
        device: *mut hackrf_device,
        callback: hackrf_sample_block_cb_fn,
        tx_ctx: *mut c_void,
    ) -> c_int;
    pub fn hackrf_enable_tx_flush(
        device: *mut hackrf_device,
        callback: hackrf_flush_cb_fn,
        flush_ctx: *mut c_void,
    ) -> c_int;
    pub fn hackrf_stop_tx(device: *mut hackrf_device) -> c_int;
    pub fn hackrf_is_streaming(device: *mut hackrf_device) -> c_int;
    pub fn hackrf_start_rx_sweep(
        device: *mut hackrf_device,
        callback: hackrf_sample_block_cb_fn,
        rx_ctx: *mut c_void,
    ) -> c_int;
    pub fn hackrf_init_sweep(
        device: *mut hackrf_device,
        frequency_list: *const u16,
        num_ranges: c_int,
        num_bytes: u32,
        step_width: u32,
        offset: u32,
        style: sweep_style,
    ) -> c_int;
    pub fn hackrf_set_tx_underrun_limit(device: *mut hackrf_device, value: u32) -> c_int;
    pub fn hackrf_set_rx_overrun_limit(device: *mut hackrf_device, value: u32) -> c_int;
    pub fn hackrf_set_hw_sync_mode(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_get_transfer_buffer_size(device: *mut hackrf_device) -> usize;
    pub fn hackrf_get_transfer_queue_depth(device: *mut hackrf_device) -> u32;

    // Device / board info
    pub fn hackrf_board_id_read(device: *mut hackrf_device, value: *mut u8) -> c_int;
    pub fn hackrf_version_string_read(
        device: *mut hackrf_device,
        version: *mut c_char,
        length: u8,
    ) -> c_int;
    pub fn hackrf_usb_api_version_read(device: *mut hackrf_device, version: *mut u16) -> c_int;
    pub fn hackrf_board_partid_serialno_read(
        device: *mut hackrf_device,
        read_partid_serialno: *mut read_partid_serialno_t,
    ) -> c_int;
    pub fn hackrf_board_rev_read(device: *mut hackrf_device, value: *mut u8) -> c_int;
    pub fn hackrf_supported_platform_read(device: *mut hackrf_device, value: *mut u32) -> c_int;
    pub fn hackrf_reset(device: *mut hackrf_device) -> c_int;
    pub fn hackrf_set_ui_enable(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_set_leds(device: *mut hackrf_device, state: u8) -> c_int;

    // Tuning / gain / sample rate / filtering
    pub fn hackrf_set_freq(device: *mut hackrf_device, freq_hz: u64) -> c_int;
    pub fn hackrf_set_freq_explicit(
        device: *mut hackrf_device,
        if_freq_hz: u64,
        lo_freq_hz: u64,
        path: rf_path_filter,
    ) -> c_int;
    pub fn hackrf_set_sample_rate_manual(
        device: *mut hackrf_device,
        freq_hz: u32,
        divider: u32,
    ) -> c_int;
    pub fn hackrf_set_sample_rate(device: *mut hackrf_device, freq_hz: f64) -> c_int;
    pub fn hackrf_set_baseband_filter_bandwidth(
        device: *mut hackrf_device,
        bandwidth_hz: u32,
    ) -> c_int;
    pub fn hackrf_set_amp_enable(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_set_lna_gain(device: *mut hackrf_device, value: u32) -> c_int;
    pub fn hackrf_set_vga_gain(device: *mut hackrf_device, value: u32) -> c_int;
    pub fn hackrf_set_txvga_gain(device: *mut hackrf_device, value: u32) -> c_int;
    pub fn hackrf_set_antenna_enable(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_set_clkout_enable(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_get_clkin_status(device: *mut hackrf_device, status: *mut u8) -> c_int;
    pub fn hackrf_set_user_bias_t_opts(
        device: *mut hackrf_device,
        req: *mut hackrf_bias_t_user_settting_req,
    ) -> c_int;
    pub fn hackrf_set_p1_ctrl(device: *mut hackrf_device, signal: p1_ctrl_signal) -> c_int;
    pub fn hackrf_set_p2_ctrl(device: *mut hackrf_device, signal: p2_ctrl_signal) -> c_int;
    pub fn hackrf_set_clkin_ctrl(device: *mut hackrf_device, signal: clkin_ctrl_signal) -> c_int;
    pub fn hackrf_set_narrowband_filter(device: *mut hackrf_device, value: u8) -> c_int;
    pub fn hackrf_set_fpga_bitstream(device: *mut hackrf_device, index: u8) -> c_int;

    // Opera Cake
    pub fn hackrf_get_operacake_boards(device: *mut hackrf_device, boards: *mut u8) -> c_int;
    pub fn hackrf_set_operacake_mode(
        device: *mut hackrf_device,
        address: u8,
        mode: operacake_switching_mode,
    ) -> c_int;
    pub fn hackrf_get_operacake_mode(
        device: *mut hackrf_device,
        address: u8,
        mode: *mut operacake_switching_mode,
    ) -> c_int;
    pub fn hackrf_set_operacake_ports(
        device: *mut hackrf_device,
        address: u8,
        port_a: u8,
        port_b: u8,
    ) -> c_int;
    pub fn hackrf_set_operacake_dwell_times(
        device: *mut hackrf_device,
        dwell_times: *mut hackrf_operacake_dwell_time,
        count: u8,
    ) -> c_int;
    pub fn hackrf_set_operacake_freq_ranges(
        device: *mut hackrf_device,
        freq_ranges: *mut hackrf_operacake_freq_range,
        count: u8,
    ) -> c_int;
    pub fn hackrf_operacake_gpio_test(
        device: *mut hackrf_device,
        address: u8,
        test_result: *mut u16,
    ) -> c_int;

    // Enum -> string helpers
    pub fn hackrf_error_name(errcode: hackrf_error) -> *const c_char;
    pub fn hackrf_board_id_name(board_id: hackrf_board_id) -> *const c_char;
    pub fn hackrf_board_id_platform(board_id: hackrf_board_id) -> u32;
    pub fn hackrf_usb_board_id_name(usb_board_id: hackrf_usb_board_id) -> *const c_char;
    pub fn hackrf_filter_path_name(path: rf_path_filter) -> *const c_char;
    pub fn hackrf_board_rev_name(board_rev: hackrf_board_rev) -> *const c_char;
    pub fn hackrf_compute_baseband_filter_bw_round_down_lt(bandwidth_hz: u32) -> u32;
    pub fn hackrf_compute_baseband_filter_bw(bandwidth_hz: u32) -> u32;
}

// Extra entry points from the Android-patched libhackrf (opened from a USB fd).
#[cfg(target_os = "android")]
unsafe extern "C" {
    /// Android replacement for `hackrf_init` (skips libusb device discovery).
    pub fn hackrf_init_on_android() -> c_int;

    /// Open a device from an already-open USB file descriptor obtained via the
    /// Android `UsbManager`.
    pub fn hackrf_open_on_android(fd: c_int, device: *mut *mut hackrf_device) -> c_int;
}
