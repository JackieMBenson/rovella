#[cfg(target_os = "windows")]
mod plat_libs {
    pub use winapi::ctypes::c_int;
    pub use winapi::shared::minwindef::*;
    pub use winapi::shared::windef::POINT;
    pub use winapi::shared::windef::*;
    pub use winapi::shared::windowsx::{GET_X_LPARAM, GET_Y_LPARAM};
    pub use winapi::um::errhandlingapi::GetLastError;
    pub use winapi::um::libloaderapi::*;
    pub use winapi::um::synchapi::Sleep;
    pub use winapi::um::winuser::*;

    pub use std::alloc::{alloc_zeroed, dealloc, Layout};
    pub use std::borrow::Borrow;
    pub use std::ffi::CString;
    pub use std::ops::Deref;
    pub use std::ptr;
    pub use std::ptr::null_mut;
}

#[cfg(target_os = "linux")]
mod plat_libs {
    pub use std::ptr;
    pub use std::ptr::{null, null_mut};
    pub use std::{thread, time};
    pub use x11::*;
    pub use xcb::ffi::xcb_connection_t;
    pub use xcb::ffi::xproto::*;
    pub use xcb::ffi::*;
    pub use xcb::ffi::{xcb_flush, xcb_generic_event_t, xcb_poll_for_event};
    pub use xcb::*;
    pub use xcb::{ConnResult, Connection};
}

use crate::event::{Event, EventData, EventDeque, EventType};
use crate::keys::Key;
use plat_libs::*;

/// Causes the current thread to sleep for a certain amount of milliseconds
#[cfg(target_os = "windows")]
#[inline]
pub fn sleep(ms: u32) {
    unsafe {
        Sleep(ms);
    }
}

/// A struct representative of the window
#[allow(dead_code)]
pub struct Window {
    plat_win: PlatformWindow,
    width: u16,
    height: u16,
    x: i16,
    y: i16,
}

impl Window {
    /// creates a value for all variables in a 'Window"
    #[inline]
    pub fn new(name: &'static str, width: u16, height: u16, x: i16, y: i16) -> Option<Window> {
        let plat_win = PlatformWindow::new(name, width, height, x, y);

        if plat_win.is_none() {
            log_fatal!("Platform window couldn't be created");
            return None;
        }

        return Some(Window {
            plat_win: plat_win.unwrap(),
            width,
            height,
            x,
            y,
        });
    }

    /// Gets events and helps to send them to the event manager
    /// Its important that &self is used here since
    #[inline]
    pub fn update(&self, ev_que: &mut EventDeque) {
        self.plat_win.update(ev_que);
    }

    /// Frees up memory and calls shutdown functions
    #[inline]
    pub fn shutdown(&self) {
        self.plat_win.destroy();
    }
}

/// A struct for platform related aspects of a window
#[cfg(target_os = "windows")]
pub struct PlatformWindow {
    hinst: *mut HINSTANCE__,
    hwnd: *mut HWND__,
}

pub trait TPlatformWindow {
    fn new(name: &'static str, width: u16, height: u16, x: i16, y: i16) -> Option<PlatformWindow>;
    fn update(&self, ev_que: &mut EventDeque);
    fn destroy(&self);
}

#[cfg(target_os = "windows")]
impl TPlatformWindow for PlatformWindow {
    /// creates the window
    fn new(name: &'static str, width: u16, height: u16, x: i16, y: i16) -> Option<PlatformWindow> {
        let mut win = PlatformWindow {
            hinst: null_mut(),
            hwnd: null_mut(),
        };

        unsafe {
            win.hinst = GetModuleHandleA(0 as *const i8);
            let icon = LoadIconA(win.hinst, IDI_APPLICATION as *const i8);
            let cursor = LoadCursorA(win.hinst, IDC_ARROW as *const i8);

            let class_name = CString::new("rovella_window_class").expect("CString ERROR");
            let window_name = CString::new(name).expect("CString ERROR");

            let wc = WNDCLASSA {
                style: CS_DBLCLKS,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 1,
                hInstance: win.hinst,
                hIcon: icon,
                hCursor: cursor,
                hbrBackground: null_mut(),
                lpszMenuName: null_mut(),
                lpszClassName: class_name.deref().as_ptr(),
            };

            if RegisterClassA(&wc) == 0 {
                log_fatal!("failed to register window class");
                return None;
            }

            let window_style = WS_OVERLAPPED | WS_SYSMENU;
            let window_ex_style = WS_EX_APPWINDOW | WS_MAXIMIZEBOX | WS_MINIMIZEBOX;

            let layout = Layout::new::<RECT>();
            let border_rect: *mut u8 = alloc_zeroed(layout);

            AdjustWindowRectEx(border_rect as *mut RECT, window_style, 0, window_ex_style);

            dealloc(border_rect, layout);

            win.hwnd = CreateWindowExA(
                window_ex_style,
                class_name.deref().as_ptr(),
                window_name.deref().as_ptr(),
                window_style,
                x as c_int,
                y as c_int,
                width as c_int,
                height as c_int,
                null_mut(),
                null_mut(),
                win.hinst,
                null_mut(),
            );

            if win.hwnd.is_null() {
                log_fatal!("Failed to create window {}", name);
                return None;
            }

            ShowWindow(win.hwnd, SW_SHOW);
        }

        return Some(win);
    }

    #[inline]
    fn update(&self, ev_que: &mut EventDeque) {
        unsafe {
            SetWindowLongPtrA(self.hwnd, GWLP_USERDATA, ptr::addr_of_mut!(*ev_que) as _);
        }

        let mut message: MSG = MSG {
            hwnd: null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: POINT { x: 0, y: 0 },
        };

        unsafe {
            while PeekMessageA(ptr::addr_of_mut!(message), null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(ptr::addr_of_mut!(message));
                DispatchMessageA(ptr::addr_of_mut!(message));
            }
        }
    }

    /// destroys the window
    fn destroy(&self) {
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        } else {
            log_warn!("Attempted to close HWND with null value");
        }
    }
}

#[cfg(target_os = "windows")]
impl From<u32> for EventType {
    /// converts a u32 to EventType and vice versa
    fn from(msg: u32) -> Self {
        match msg {
            WM_CLOSE => EventType::WinClose,
            WM_SHOWWINDOW => EventType::WinShow,
            WM_SIZE => EventType::WinResize,
            WM_KEYDOWN => EventType::KeyDown,
            WM_SYSKEYDOWN => EventType::KeyDown,
            WM_KEYUP => EventType::KeyUp,
            WM_SYSKEYUP => EventType::KeyUp,
            WM_MOUSEMOVE => EventType::MouseMove,
            WM_MOUSEWHEEL => EventType::MouseWheel,
            WM_LBUTTONDOWN => EventType::MouseLeftBtnDown,
            WM_MBUTTONDOWN => EventType::MouseMidBtnDown,
            WM_RBUTTONDOWN => EventType::MouseRightBtnDown,
            WM_LBUTTONUP => EventType::MouseLeftBtnUp,
            WM_MBUTTONUP => EventType::MouseMidBtnUp,
            WM_RBUTTONUP => EventType::MouseRightBtnUp,
            _ => EventType::None,
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn add_event_to_que(event: Event, hwnd: *mut HWND__) {
    let ev_que: *mut EventDeque = GetWindowLongPtrA(hwnd, GWLP_USERDATA) as _;

    if !(ev_que.is_null()) {
        (*ev_que).push_back(event);
    } else {
        let err = GetLastError();
        if err != 0 {
            log_error!(
                "window proc couldn't retrieve event queue with error code, {} ",
                err
            );
        }
    }
}

/// the callback for window event management used in the win32 api
#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: *mut HWND__,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> LRESULT {
    if msg == WM_CREATE {
        return DefWindowProcA(hwnd, msg, wparam, lparam);
    }

    match msg {
        WM_ERASEBKGND => {
            return 1;
        }
        WM_CLOSE => {
            add_event_to_que(
                Event {
                    e_type: EventType::WinClose,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
            return 0;
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            add_event_to_que(
                Event {
                    e_type: EventType::KeyDown,
                    data0: EventData {
                        unsigned: wparam as u16,
                    },
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_KEYUP | WM_SYSKEYUP => {
            add_event_to_que(
                Event {
                    e_type: EventType::KeyUp,
                    data0: EventData {
                        unsigned: wparam as u16,
                    },
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_MOUSEMOVE => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseMove,
                    data0: EventData {
                        signed: GET_X_LPARAM(lparam) as i16,
                    },
                    data1: EventData {
                        signed: GET_Y_LPARAM(lparam) as i16,
                    },
                },
                hwnd,
            );
        }
        WM_MOUSEWHEEL => {
            let z_delta = GET_WHEEL_DELTA_WPARAM(wparam);
            if z_delta != 0 {
                if z_delta < 0 {
                    add_event_to_que(
                        Event {
                            e_type: EventType::MouseWheel,
                            data0: EventData { signed: -1 as i16 },
                            data1: EventData::default(),
                        },
                        hwnd,
                    );
                } else {
                    add_event_to_que(
                        Event {
                            e_type: EventType::MouseWheel,
                            data0: EventData { signed: 1 as i16 },
                            data1: EventData::default(),
                        },
                        hwnd,
                    );
                }
            }
        }
        WM_LBUTTONDOWN => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseLeftBtnDown,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_MBUTTONDOWN => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseMidBtnDown,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_RBUTTONDOWN => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseRightBtnDown,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_LBUTTONUP => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseLeftBtnUp,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_MBUTTONUP => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseMidBtnUp,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        WM_RBUTTONUP => {
            add_event_to_que(
                Event {
                    e_type: EventType::MouseRightBtnUp,
                    data0: EventData::default(),
                    data1: EventData::default(),
                },
                hwnd,
            );
        }
        _ => {}
    }

    return DefWindowProcA(hwnd, msg, wparam, lparam);
}

#[cfg(target_os = "windows")]
impl From<u16> for Key {
    fn from(val: u16) -> Self {
        return match val {
            0x08 => Key::Backspace,
            0x0D => Key::Enter,
            0x09 => Key::Tab,
            0x10 => Key::Shift,
            0x11 => Key::Control,
            0x13 => Key::Pause,
            0x14 => Key::Capital,
            0x1B => Key::Escape,
            0x1C => Key::Convert,
            0x1D => Key::NonConvert,
            0x1E => Key::Accept,
            0x1F => Key::ModeChange,
            0x20 => Key::Space,
            0x21 => Key::Prior,
            0x22 => Key::Next,
            0x23 => Key::End,
            0x24 => Key::Home,
            0x25 => Key::Left,
            0x26 => Key::Up,
            0x27 => Key::Right,
            0x28 => Key::Down,
            0x29 => Key::Select,
            0x2A => Key::Print,
            0x2B => Key::Execute,
            0x2C => Key::Snapshot,
            0x2D => Key::Insert,
            0x2E => Key::Delete,
            0x2F => Key::Help,
            0x41 => Key::A,
            0x42 => Key::B,
            0x43 => Key::C,
            0x44 => Key::D,
            0x45 => Key::E,
            0x46 => Key::F,
            0x47 => Key::G,
            0x48 => Key::H,
            0x49 => Key::I,
            0x4A => Key::J,
            0x4B => Key::K,
            0x4C => Key::L,
            0x4D => Key::M,
            0x4E => Key::N,
            0x4F => Key::O,
            0x50 => Key::P,
            0x51 => Key::Q,
            0x52 => Key::R,
            0x53 => Key::S,
            0x54 => Key::T,
            0x55 => Key::U,
            0x56 => Key::V,
            0x57 => Key::W,
            0x58 => Key::X,
            0x59 => Key::Y,
            0x5A => Key::Z,
            0x30 => Key::N0,
            0x31 => Key::N1,
            0x32 => Key::N2,
            0x33 => Key::N3,
            0x34 => Key::N4,
            0x35 => Key::N5,
            0x36 => Key::N6,
            0x37 => Key::N7,
            0x38 => Key::N8,
            0x39 => Key::N9,
            0x5B => Key::Lwin,
            0x5C => Key::Rwin,
            0x5D => Key::Apps,
            0x5F => Key::Sleep,
            0x60 => Key::Numpad0,
            0x61 => Key::Numpad1,
            0x62 => Key::Numpad2,
            0x63 => Key::Numpad3,
            0x64 => Key::Numpad4,
            0x65 => Key::Numpad5,
            0x66 => Key::Numpad6,
            0x67 => Key::Numpad7,
            0x68 => Key::Numpad8,
            0x69 => Key::Numpad9,
            0x6A => Key::Multiply,
            0x6B => Key::Add,
            0x6C => Key::Separator,
            0x6D => Key::Subtract,
            0x6E => Key::Decimal,
            0x6F => Key::Divide,
            0x70 => Key::F1,
            0x71 => Key::F2,
            0x72 => Key::F3,
            0x73 => Key::F4,
            0x74 => Key::F5,
            0x75 => Key::F6,
            0x76 => Key::F7,
            0x77 => Key::F8,
            0x78 => Key::F9,
            0x79 => Key::F10,
            0x7A => Key::F11,
            0x7B => Key::F12,
            0x7C => Key::F13,
            0x7D => Key::F14,
            0x7E => Key::F15,
            0x7F => Key::F16,
            0x80 => Key::F17,
            0x81 => Key::F18,
            0x82 => Key::F19,
            0x83 => Key::F20,
            0x84 => Key::F21,
            0x85 => Key::F22,
            0x86 => Key::F23,
            0x87 => Key::F24,
            0x90 => Key::Numlock,
            0x91 => Key::ScrollLock,
            0x92 => Key::NumpadEqual,
            0xA0 => Key::LShift,
            0xA1 => Key::RShift,
            0xA2 => Key::LControl,
            0xA3 => Key::RControl,
            0xA4 => Key::LAlt,
            0xA5 => Key::RAlt,
            0xBA => Key::Semicolon,
            0xBB => Key::Plus,
            0xBC => Key::Comma,
            0xBD => Key::Minus,
            0xBE => Key::Period,
            0xBF => Key::Slash,
            0xC0 => Key::Grave,
            _ => Key::None,
        };
    }
}

#[cfg(target_os = "linux")]
impl From<u16> for Key {
    fn from(val: u16) -> Self {
        return match val {
            XK_Escape => Key::Escape,
            _ => Key::None,
        };
    }
}

#[cfg(target_os = "linux")]
#[inline]
pub fn sleep(ms: u32) {
    thread::sleep(time::Duration::from_millis(ms as u64));
}

#[cfg(target_os = "linux")]
pub struct PlatformWindow {
    display: *mut xlib::Display,
    connection: *mut xcb_connection_t,
    window: u32,
    screen: *mut xcb_screen_t,
    wm_protocols: xcb_atom_t,
    wm_delete_win: xcb_atom_t,
}

#[cfg(target_os = "linux")]
impl TPlatformWindow for PlatformWindow {
    /// creates the window
    fn new(name: &'static str, width: u16, height: u16, x: i16, y: i16) -> Option<PlatformWindow> {
        unsafe {
            let display = xlib::XOpenDisplay(null());

            if display.is_null() {
                log_fatal!("Could not get display");
                return None;
            }

            xlib::XAutoRepeatOff(display);

            let connection: *mut xcb_connection_t =
                x11::xlib_xcb::XGetXCBConnection(display) as *mut xcb_connection_t;

            if xcb_connection_has_error(connection) != 0 {
                log_fatal!("Unable to connect to X server, have you set one up?");
                return None;
            }

            let setup = xcb_get_setup(connection);
            let screen: *mut xcb_screen_t;

            {
                let mut iterator = xcb_setup_roots_iterator(setup);
                screen = iterator.data;
            }

            let win: u32 = xcb_generate_id(connection);

            let event_mask = XCB_CW_BACK_PIXEL | XCB_CW_EVENT_MASK;

            let event_values: u32 = XCB_EVENT_MASK_BUTTON_PRESS
                | XCB_EVENT_MASK_BUTTON_RELEASE
                | XCB_EVENT_MASK_KEY_PRESS
                | XCB_EVENT_MASK_KEY_RELEASE
                | XCB_EVENT_MASK_EXPOSURE
                | XCB_EVENT_MASK_POINTER_MOTION
                | XCB_EVENT_MASK_STRUCTURE_NOTIFY;

            let value_list: [u32; 2] = [(*screen).black_pixel, event_values];

            let cookie = xcb_create_window(
                connection,
                XCB_COPY_FROM_PARENT as u8,
                win,
                (*screen).root,
                x,
                y,
                width,
                height,
                0,
                XCB_WINDOW_CLASS_INPUT_OUTPUT as u16,
                (*screen).root_visual,
                event_mask,
                ptr::addr_of!(value_list[0]),
            );

            xcb_change_property(
                connection,
                XCB_PROP_MODE_REPLACE as u8,
                win,
                XCB_ATOM_WM_NAME,
                XCB_ATOM_STRING,
                8 as u8,
                name.len() as u32,
                name.as_ptr() as _,
            );

            let del_str = b"WM_DELETE_WINDOW";

            let wm_delete_cookie = xcb_intern_atom(
                connection,
                0,
                del_str.len() as u16,
                del_str.as_ptr() as _
            );

            let proto_str = b"WM_PROTOCOLS";

            let wm_protocols_cookie = xcb_intern_atom(
                connection,
                0,
                proto_str.len() as u16,
                proto_str.as_ptr() as _,
            );

            let wm_delete_reply = xcb_intern_atom_reply(
                connection,
                wm_delete_cookie,
                null_mut()
            );

            let wm_proto_reply = xcb_intern_atom_reply(
                connection,
                wm_protocols_cookie,
                null_mut()
            );

            xcb_map_window(connection, win);

            let res = xcb_flush(connection);

            if res <= 0 {
                log_error!("Failed to flush stream (xcb connection)");
            }

            return Some(PlatformWindow {
                display: display,
                connection: connection,
                window: win,
                screen: screen,
                wm_protocols: (*wm_proto_reply).atom,
                wm_delete_win: (*wm_delete_reply).atom,
            });
        }
    }

    fn update(&self, ev_que: &mut EventDeque) {
        let mut event: *mut xcb_generic_event_t;
        let mut cm: *mut xcb_client_message_event_t;

        loop {
            unsafe {
                event = xcb_poll_for_event(self.connection);
                if (event.is_null()) {
                    break;
                }

                let event_enum: u8 = ((*event).response_type as u8 & 0x7f);

                match event_enum {
                    XCB_KEY_PRESS => {
                        let kb_event = event as *const xcb_key_press_event_t;

                        let key = xlib::XKeycodeToKeysym(
                            self.display,
                            (*kb_event).detail as u8,
                            (((*kb_event).detail as u32) & xlib::ShiftMask) as i32,
                        );

                        ev_que.push_back(Event {
                            e_type: EventType::KeyDown,
                            data0: EventData { unsigned: key as u16},
                            data1: EventData::default(),
                        });
                    }
                    XCB_KEY_RELEASE => {
                        let kb_event = event as *const xcb_key_press_event_t;

                        let key = xlib::XKeycodeToKeysym(
                            self.display,
                            (*kb_event).detail as u8,
                            (((*kb_event).detail as u32) & xlib::ShiftMask) as i32,
                        );

                        ev_que.push_back(Event {
                            e_type: EventType::KeyUp,
                            data0: EventData { unsigned: key as u16},
                            data1: EventData::default(),
                        });
                    }
                    XCB_MOTION_NOTIFY => {
                        let motion = event as *const xcb_motion_notify_event_t;
                        ev_que.push_back(Event {
                            e_type: EventType::MouseMove,
                            data0: EventData { signed: (*motion).root_x },
                            data1: EventData { signed: (*motion).root_y },
                        });
                    }
                    XCB_BUTTON_PRESS => {
                        let button_event = event as * mut xcb_button_press_event_t;

                        match (*button_event).detail as u32 {
                            XCB_BUTTON_INDEX_1 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseLeftBtnDown,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            XCB_BUTTON_INDEX_2 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseMidBtnDown,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            XCB_BUTTON_INDEX_3 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseRightBtnDown,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            _ => {}
                        }
                    }
                    XCB_BUTTON_RELEASE => {
                        let button_event = event as * mut xcb_button_press_event_t;

                        match (*button_event).detail as u32 {
                            XCB_BUTTON_INDEX_1 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseLeftBtnUp,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            XCB_BUTTON_INDEX_2 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseMidBtnUp,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            XCB_BUTTON_INDEX_3 => {
                                ev_que.push_back(Event {
                                    e_type: EventType::MouseRightBtnUp,
                                    data0: EventData::default(),
                                    data1: EventData::default(),
                                });
                            }
                            _ => {}
                        }
                    }
                    XCB_CLIENT_MESSAGE => {
                        cm = event as *mut xcb_client_message_event_t;

                        log_info!("Client Message");

                        if (*cm).data.data32()[0] == self.wm_delete_win {
                            ev_que.push_back(Event {
                                e_type: EventType::WinClose,
                                data0: EventData::default(),
                                data1: EventData::default(),
                            });
                        }
                    }
                    _ => {}
                }

                libc::free(event as _);
            }
        }
    }

    /// destroys the window
    fn destroy(&self) {
        unsafe {
            xlib::XAutoRepeatOn(self.display);
            xcb_destroy_window(self.connection, self.window);
        }
    }
}