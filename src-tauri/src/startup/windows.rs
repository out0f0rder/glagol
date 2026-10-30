//! Current-user Run only. The NSIS template removes the matching product value
//! on uninstall and preserves it in update mode. No StartupApproved overrides.
use super::registry_error;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::ERROR_FILE_NOT_FOUND, System::Registry::*},
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: PCWSTR = w!("Glagol"); // Keep aligned with tauri.conf.json productName / NSIS.

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: owned successful open/create, closed exactly once.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn open(path: &str, write: bool) -> Result<Option<Key>, String> {
    let path = wide(path);
    let mut key = HKEY::default();
    // SAFETY: NUL-terminated paths, valid output storage; HKCU requires no elevation.
    let result = unsafe {
        if write {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        } else {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                KEY_QUERY_VALUE,
                &mut key,
            )
        }
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    result.ok().map_err(|_| registry_error())?;
    Ok(Some(Key(key)))
}

fn read_at(path: &str) -> Result<Option<String>, String> {
    let Some(key) = open(path, false)? else {
        return Ok(None);
    };
    // One bounded call avoids unbounded allocation and size-query races. Run's
    // documented limit is 260 characters; allow one terminator and reject bad types.
    let mut data = [0u16; 261];
    let mut size = std::mem::size_of_val(&data) as u32;
    // SAFETY: aligned output buffer has exactly size bytes; API receives no other pointer.
    let result = unsafe {
        RegGetValueW(
            key.0,
            None,
            VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(data.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    result.ok().map_err(|_| registry_error())?;
    if size < 2 || !size.is_multiple_of(2) || size as usize > std::mem::size_of_val(&data) {
        return Err(registry_error());
    }
    let units = &data[..size as usize / 2];
    let Some((&0, value)) = units.split_last() else {
        return Err(registry_error());
    };
    if value.contains(&0) {
        return Err(registry_error());
    }
    String::from_utf16(value)
        .map(Some)
        .map_err(|_| registry_error())
}

fn write_at(path: &str, command: Option<&str>) -> Result<(), String> {
    if command.is_none() {
        // Do not create a key just to remove a missing value.
        let path = wide(path);
        let mut key = HKEY::default();
        // SAFETY: only open with value-write permission; no unrelated values are removed.
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                KEY_SET_VALUE,
                &mut key,
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        result.ok().map_err(|_| registry_error())?;
        let key = Key(key);
        // SAFETY: live owned key, fixed product value name only.
        let result = unsafe { RegDeleteValueW(key.0, VALUE) };
        return if result == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            result.ok().map_err(|_| registry_error())
        };
    }
    let command = wide(command.unwrap());
    let bytes: Vec<u8> = command.iter().flat_map(|v| v.to_le_bytes()).collect();
    let key = open(path, true)?.ok_or_else(registry_error)?;
    // SAFETY: REG_SZ bytes include the UTF-16 terminator; constant value name only.
    unsafe { RegSetValueExW(key.0, VALUE, None, REG_SZ, Some(&bytes)) }
        .ok()
        .map_err(|_| registry_error())
}

pub(super) fn read() -> Result<Option<String>, String> {
    read_at(RUN_KEY)
}
pub(super) fn write(command: Option<&str>) -> Result<(), String> {
    write_at(RUN_KEY, command)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn isolated_registry_roundtrip_rejects_wrong_type_and_never_touches_run() {
        // Unique test key is NOT an autostart location. RAII removes it even on panic.
        struct Fixture(String);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let path = wide(&self.0);
                // SAFETY: only the unique key owned by this test, never the Run key.
                unsafe {
                    let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()));
                }
            }
        }
        let fixture = Fixture(format!(r"Software\Glagol\Tests\{}", uuid::Uuid::new_v4()));
        assert!(read_at(&fixture.0).unwrap().is_none());
        write_at(&fixture.0, None).unwrap();
        let command =
            super::super::autostart_command(std::path::Path::new(r"C:\Тест с пробелом\glagol.exe"))
                .unwrap();
        write_at(&fixture.0, Some(&command)).unwrap();
        assert_eq!(read_at(&fixture.0).unwrap(), Some(command));
        let key = open(&fixture.0, true).unwrap().unwrap();
        let keep: Vec<u8> = wide("keep").iter().flat_map(|v| v.to_le_bytes()).collect();
        // SAFETY: malformed values go only into the disposable non-Run fixture.
        unsafe {
            RegSetValueExW(key.0, w!("unrelated"), None, REG_SZ, Some(&keep))
                .ok()
                .unwrap();
            RegSetValueExW(key.0, VALUE, None, REG_DWORD, Some(&[1, 0, 0, 0]))
                .ok()
                .unwrap();
        }
        assert!(read_at(&fixture.0).is_err());
        write_at(&fixture.0, Some(&"a".repeat(300))).unwrap();
        assert!(read_at(&fixture.0).is_err());
        write_at(&fixture.0, None).unwrap();
        write_at(&fixture.0, None).unwrap();
        assert!(read_at(&fixture.0).unwrap().is_none());
        let key = open(&fixture.0, false).unwrap().unwrap();
        // SAFETY: bounded read confirms a different value survived deletion.
        let mut data = [0u16; 16];
        let mut size = std::mem::size_of_val(&data) as u32;
        unsafe {
            RegGetValueW(
                key.0,
                None,
                w!("unrelated"),
                RRF_RT_REG_SZ,
                None,
                Some(data.as_mut_ptr().cast()),
                Some(&mut size),
            )
        }
        .ok()
        .unwrap();
        assert_eq!(&data[..5], wide("keep").as_slice());
    }
}
