use kernel32;
use shell32;
use std;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use user32;
use uuid;
use winapi;

const IDI_POLARIS_TRAY: isize = 0x102;
const UID_NOTIFICATION_ICON: u32 = 0;
const MESSAGE_NOTIFICATION_ICON: u32 = winapi::WM_USER + 1;
const MESSAGE_NOTIFICATION_ICON_QUIT: u32 = winapi::WM_USER + 2;

pub trait ToWin {
	type Out;
	fn to_win(&self) -> Self::Out;
}

impl<'a> ToWin for &'a str {
	type Out = Vec<u16>;

	fn to_win(&self) -> Self::Out {
		OsStr::new(self)
			.encode_wide()
			.chain(std::iter::once(0))
			.collect()
	}
}

impl ToWin for uuid::Uuid {
	type Out = winapi::GUID;

	fn to_win(&self) -> Self::Out {
		let bytes = self.as_bytes();
		let end = [bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
		           bytes[15]];

		winapi::GUID {
			Data1: ((bytes[0] as u32) << 24 | (bytes[1] as u32) << 16 | (bytes[2] as u32) << 8 |
			        (bytes[3] as u32)),
			Data2: ((bytes[4] as u16) << 8 | (bytes[5] as u16)),
			Data3: ((bytes[6] as u16) << 8 | (bytes[7] as u16)),
			Data4: end,
		}
	}
}

pub trait Constructible {
	type Out;
	fn new() -> Self::Out;
}

impl Constructible for winapi::NOTIFYICONDATAW {
	type Out = winapi::NOTIFYICONDATAW;

	fn new() -> Self::Out {
		winapi::NOTIFYICONDATAW {
			cbSize: std::mem::size_of::<winapi::NOTIFYICONDATAW>() as u32,
			hWnd: std::ptr::null_mut(),
			uFlags: 0,
			guidItem: uuid::Uuid::nil().to_win(),
			hIcon: std::ptr::null_mut(),
			uID: 0,
			uCallbackMessage: 0,
			szTip: [0; 128],
			dwState: 0,
			dwStateMask: 0,
			szInfo: [0; 256],
			uTimeout: winapi::NOTIFYICON_VERSION_4,
			szInfoTitle: [0; 64],
			dwInfoFlags: 0,
			hBalloonIcon: std::ptr::null_mut(),
		}
	}
}

fn create_window() -> Option<winapi::HWND> {

	let class_name = "Polaris-class".to_win();
	let window_name = "Polaris-window".to_win();

	unsafe {
		let module_handle = kernel32::GetModuleHandleW(std::ptr::null());
		let wnd = winapi::WNDCLASSW {
			style: 0,
			lpfnWndProc: Some(window_proc),
			hInstance: module_handle,
			hIcon: std::ptr::null_mut(),
			hCursor: std::ptr::null_mut(),
			lpszClassName: class_name.as_ptr(),
			hbrBackground: winapi::COLOR_WINDOW as winapi::HBRUSH,
			lpszMenuName: std::ptr::null_mut(),
			cbClsExtra: 0,
			cbWndExtra: 0,
		};

		let atom = user32::RegisterClassW(&wnd);
		if atom == 0 {
			return None;
		}

		let window_handle = user32::CreateWindowExW(0,
		                                            atom as winapi::LPCWSTR,
		                                            window_name.as_ptr(),
		                                            winapi::WS_DISABLED,
		                                            0,
		                                            0,
		                                            0,
		                                            0,
		                                            user32::GetDesktopWindow(),
		                                            std::ptr::null_mut(),
		                                            std::ptr::null_mut(),
		                                            std::ptr::null_mut());

		if window_handle.is_null() {
			return None;
		}

		return Some(window_handle);
	}
}

fn add_notification_icon(window: winapi::HWND) {

	let mut tooltip = [0 as winapi::WCHAR; 128];
	for (&x, p) in "Polaris".to_win().iter().zip(tooltip.iter_mut()) {
		*p = x;
	}

	unsafe {
		let module = kernel32::GetModuleHandleW(std::ptr::null());
		let icon = user32::LoadIconW(module, std::mem::transmute(IDI_POLARIS_TRAY));
		let mut flags = winapi::NIF_MESSAGE | winapi::NIF_TIP;
		if !icon.is_null() {
			flags |= winapi::NIF_ICON;
		}

		let mut icon_data = winapi::NOTIFYICONDATAW::new();
		icon_data.hWnd = window;
		icon_data.uID = UID_NOTIFICATION_ICON;
		icon_data.uFlags = flags;
		icon_data.hIcon = icon;
		icon_data.uCallbackMessage = MESSAGE_NOTIFICATION_ICON;
		icon_data.szTip = tooltip;

		shell32::Shell_NotifyIconW(winapi::NIM_ADD, &mut icon_data);
	}
}

fn remove_notification_icon(window: winapi::HWND) {
	let mut icon_data = winapi::NOTIFYICONDATAW::new();
	icon_data.hWnd = window;
	icon_data.uID = UID_NOTIFICATION_ICON;
	unsafe {
		shell32::Shell_NotifyIconW(winapi::NIM_DELETE, &mut icon_data);
	}
}

fn open_notification_context_menu(window: winapi::HWND) {
	println!("Opening notification icon context menu");
	let quit_string = "Quit Polaris".to_win();

	unsafe {
		let context_menu = user32::CreatePopupMenu();
		if context_menu.is_null() {
			return;
		}
		user32::InsertMenuW(context_menu,
		                    0,
		                    winapi::winuser::MF_STRING,
		                    MESSAGE_NOTIFICATION_ICON_QUIT as u64,
		                    quit_string.as_ptr());

		let mut cursor_position = winapi::POINT { x: 0, y: 0 };
		user32::GetCursorPos(&mut cursor_position);

		user32::SetForegroundWindow(window);
		let flags = winapi::winuser::TPM_RIGHTALIGN | winapi::winuser::TPM_BOTTOMALIGN |
		            winapi::winuser::TPM_RIGHTBUTTON;
		user32::TrackPopupMenu(context_menu,
		                       flags,
		                       cursor_position.x,
		                       cursor_position.y,
		                       0,
		                       window,
		                       std::ptr::null_mut());
		user32::PostMessageW(window, 0, 0, 0);

		println!("Closing notification context menu");
		user32::DestroyMenu(context_menu);
	}
}

fn quit(window: winapi::HWND) {
	println!("Shutting down UI");
	unsafe {
		user32::PostMessageW(window, winapi::winuser::WM_CLOSE, 0, 0);
	}
}

pub fn run() {
	println!("Starting up UI (Windows)");

	create_window().expect("Could not initialize window");

	let mut message = winapi::MSG {
		hwnd: std::ptr::null_mut(),
		message: 0,
		wParam: 0,
		lParam: 0,
		time: 0,
		pt: winapi::POINT { x: 0, y: 0 },
	};

	loop {
		let status: i32;
		unsafe {
			status = user32::GetMessageW(&mut message, std::ptr::null_mut(), 0, 0);
			if status == -1 {
				panic!("GetMessageW error: {}", kernel32::GetLastError());
			}
			if status == 0 {
				break;
			}
			user32::TranslateMessage(&message);
			user32::DispatchMessageW(&message);
		}
	}
}

pub unsafe extern "system" fn window_proc(window: winapi::HWND,
                                          msg: winapi::UINT,
                                          w_param: winapi::WPARAM,
                                          l_param: winapi::LPARAM)
                                          -> winapi::LRESULT {
	match msg {

		winapi::winuser::WM_CREATE => {
			add_notification_icon(window);
		}

		MESSAGE_NOTIFICATION_ICON => {
			match winapi::LOWORD(l_param as winapi::DWORD) as u32 {
				winapi::winuser::WM_RBUTTONUP => {
					open_notification_context_menu(window);
				}
				_ => (),
			}
		}

		winapi::winuser::WM_COMMAND => {
			match winapi::LOWORD(w_param as winapi::DWORD) as u32 {
				MESSAGE_NOTIFICATION_ICON_QUIT => {
					quit(window);
				}
				_ => (),
			}
		}

		winapi::winuser::WM_DESTROY => {
			remove_notification_icon(window);
			user32::PostQuitMessage(0);
		}

		_ => (),
	};

	return user32::DefWindowProcW(window, msg, w_param, l_param);
}
