const DPAPI_PREFIX: &str = "dpapi:v1:";

pub fn protect_secret(value: &str) -> Result<String, String> {
    if value.is_empty() || value.starts_with(DPAPI_PREFIX) {
        return Ok(value.to_string());
    }

    #[cfg(target_os = "windows")]
    {
        windows_dpapi::protect(value.as_bytes())
            .map(|bytes| format!("{DPAPI_PREFIX}{}", encode_hex(&bytes)))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(value.to_string())
    }
}

pub fn expose_secret(value: &str) -> Result<String, String> {
    let Some(encoded) = value.strip_prefix(DPAPI_PREFIX) else {
        return Ok(value.to_string());
    };

    #[cfg(target_os = "windows")]
    {
        let encrypted = decode_hex(encoded)?;
        let plain = windows_dpapi::unprotect(&encrypted)?;
        String::from_utf8(plain).map_err(|error| format!("解码受保护凭据失败: {error}"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = encoded;
        Err("该凭据由 Windows DPAPI 保护，无法在当前系统解密".to_string())
    }
}

pub fn is_protected(value: &str) -> bool {
    value.starts_with(DPAPI_PREFIX)
}

#[cfg(target_os = "windows")]
fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(target_os = "windows")]
fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if value.len() % 2 != 0 {
        return Err("受保护凭据的十六进制长度无效".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = decode_nibble(pair[0])?;
            let low = decode_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn decode_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("受保护凭据包含无效的十六进制字符".to_string()),
    }
}

#[cfg(target_os = "windows")]
mod windows_dpapi {
    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        fn CryptProtectData(
            input: *const DataBlob,
            description: *const u16,
            entropy: *const DataBlob,
            reserved: *mut std::ffi::c_void,
            prompt: *mut std::ffi::c_void,
            flags: u32,
            output: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            input: *const DataBlob,
            description: *mut *mut u16,
            entropy: *const DataBlob,
            reserved: *mut std::ffi::c_void,
            prompt: *mut std::ffi::c_void,
            flags: u32,
            output: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    pub fn protect(value: &[u8]) -> Result<Vec<u8>, String> {
        crypt(value, true)
    }

    pub fn unprotect(value: &[u8]) -> Result<Vec<u8>, String> {
        crypt(value, false)
    }

    fn crypt(value: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
        let input = DataBlob {
            cb_data: value.len() as u32,
            pb_data: value.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let succeeded = unsafe {
            if encrypt {
                CryptProtectData(
                    &input,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    CRYPTPROTECT_UI_FORBIDDEN,
                    &mut output,
                )
            }
        };
        if succeeded == 0 || output.pb_data.is_null() {
            return Err("Windows DPAPI 凭据保护操作失败".to_string());
        }
        let result = unsafe {
            let slice = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize);
            let result = slice.to_vec();
            LocalFree(output.pb_data as *mut std::ffi::c_void);
            result
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{expose_secret, is_protected, protect_secret};

    #[test]
    fn protected_secret_round_trips() {
        let protected = protect_secret("sk-test-secret").expect("protect");
        #[cfg(target_os = "windows")]
        {
            assert!(is_protected(&protected));
            assert!(!protected.contains("sk-test-secret"));
        }
        assert_eq!(expose_secret(&protected).expect("expose"), "sk-test-secret");
    }
}
