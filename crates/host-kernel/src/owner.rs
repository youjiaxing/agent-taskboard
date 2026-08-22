use std::fs;
use std::io;
use std::path::Path;

use crate::KernelError;

pub(crate) fn restrict_to_owner(path: &Path) -> Result<(), KernelError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        windows_acl::restrict(path).map_err(KernelError::from)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(KernelError::Protocol(
            "cannot restrict secret file permissions on this platform".into(),
        ))
    }
}

pub(crate) fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    #[cfg(windows)]
    if to.exists() {
        fs::remove_file(to)?;
    }
    fs::rename(from, to)
}

#[cfg(windows)]
mod windows_acl {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        CloseHandle, LocalFree, BOOL, GENERIC_READ, GENERIC_WRITE, HANDLE, WIN32_ERROR,
    };
    use windows_sys::Win32::Security::Authorization::{
        SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE,
        SET_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenUser, PSID, TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const DACL_SECURITY_INFORMATION: u32 = 0x0000_0004;
    const PROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x8000_0000;
    const NO_INHERITANCE: u32 = 0;
    const DELETE: u32 = 0x0001_0000;
    const WRITE_DAC: u32 = 0x0004_0000;

    pub(super) fn restrict(path: &Path) -> io::Result<()> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        unsafe { restrict_wide(wide.as_ptr()) }
    }

    unsafe fn restrict_wide(path: *const u16) -> io::Result<()> {
        let mut token: HANDLE = ptr::null_mut();
        let opened: BOOL = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token);
        if opened == 0 {
            return Err(io::Error::last_os_error());
        }
        struct Handle(HANDLE);
        impl Drop for Handle {
            fn drop(&mut self) {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
        let token = Handle(token);

        let mut needed = 0u32;
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut needed);
        if needed == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut buffer = vec![0u8; needed as usize];
        let got: BOOL = GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        );
        if got == 0 {
            return Err(io::Error::last_os_error());
        }
        let user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let sid: PSID = user.User.Sid;

        let access = EXPLICIT_ACCESS_W {
            grfAccessPermissions: GENERIC_READ | GENERIC_WRITE | DELETE | WRITE_DAC,
            grfAccessMode: SET_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: ptr::null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_USER,
                ptstrName: sid.cast(),
            },
        };

        let mut acl = ptr::null_mut();
        let status: WIN32_ERROR = SetEntriesInAclW(1, &access, ptr::null(), &mut acl);
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        struct Local(*mut core::ffi::c_void);
        impl Drop for Local {
            fn drop(&mut self) {
                unsafe {
                    LocalFree(self.0);
                }
            }
        }
        let _acl = Local(acl.cast());

        let status: WIN32_ERROR = SetNamedSecurityInfoW(
            path,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            acl,
            ptr::null(),
        );
        if status != 0 {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        Ok(())
    }
}
