/* SPDX-License-Identifier: Apache-2.0 */
#include <stdint.h>

#if defined(_WIN32)
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <aclapi.h>
#include <wchar.h>
#include <stddef.h>

static int trusted_sid(PSID sid, PSID user, PSID system, PSID administrators) {
    return IsValidSid(sid) && (EqualSid(sid, user) || EqualSid(sid, system) || EqualSid(sid, administrators));
}

int kairo_fs_guard(uintptr_t raw, int private_directory) {
    HANDLE directory = (HANDLE)raw, token = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    PACL acl = NULL;
    PSID owner = NULL;
    DWORD size = 0, result;
    TOKEN_USER *user = NULL;
    union { DWORD alignment; BYTE bytes[SECURITY_MAX_SID_SIZE]; } system, administrators;
    DWORD system_size = (DWORD)sizeof(system.bytes), admin_size = (DWORD)sizeof(administrators.bytes);
    FILE_ATTRIBUTE_TAG_INFO attributes;
    WCHAR path[32768], filesystem[32];
    DWORD volume_flags;
    int verdict = 4;
    if (!GetFileInformationByHandleEx(directory, FileAttributeTagInfo, &attributes, sizeof(attributes))) return 4;
    if (!(attributes.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) || (attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT)) return 6;
    size = GetFinalPathNameByHandleW(directory, path, 32768, FILE_NAME_NORMALIZED);
    if (!size || size >= 32768) return 4;
    if (_wcsnicmp(path, L"\\\\?\\UNC\\", 8) == 0) return 5;
    if (!GetVolumeInformationByHandleW(directory, NULL, 0, NULL, NULL, &volume_flags, filesystem, 32)) return 4;
    if (!(volume_flags & FILE_PERSISTENT_ACLS) || (wcscmp(filesystem, L"NTFS") && wcscmp(filesystem, L"ReFS"))) return 5;
    result = GetSecurityInfo(directory, SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION, &owner, NULL, &acl, NULL, &descriptor);
    if (result != ERROR_SUCCESS || !descriptor || !owner || !acl || !IsValidSecurityDescriptor(descriptor) || !IsValidAcl(acl)) goto done;
    if (!OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, TRUE, &token)) {
        if (GetLastError() != ERROR_NO_TOKEN || !OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) goto done;
    }
    size = 0;
    GetTokenInformation(token, TokenUser, NULL, 0, &size);
    if (!size || GetLastError() != ERROR_INSUFFICIENT_BUFFER) goto done;
    user = (TOKEN_USER *)LocalAlloc(LMEM_FIXED, size);
    if (!user || !GetTokenInformation(token, TokenUser, user, size, &size)) goto done;
    if (!EqualSid(owner, user->User.Sid)) { verdict = 1; goto done; }
    if (!CreateWellKnownSid(WinLocalSystemSid, NULL, system.bytes, &system_size) || !CreateWellKnownSid(WinBuiltinAdministratorsSid, NULL, administrators.bytes, &admin_size)) goto done;
    verdict = 0;
    for (DWORD i = 0; i < acl->AceCount; i++) {
        void *entry = NULL;
        if (!GetAce(acl, i, &entry)) { verdict = 4; break; }
        ACE_HEADER *header = (ACE_HEADER *)entry;
        /* Include inherit-only grants: they can become effective on newly
           created staging files even while absent from directory access. */
        if (header->AceType == ACCESS_DENIED_ACE_TYPE) continue;
        if (header->AceType != ACCESS_ALLOWED_ACE_TYPE) { verdict = 3; break; }
        if (header->AceSize < offsetof(ACCESS_ALLOWED_ACE, SidStart) + 8) { verdict = 4; break; }
        ACCESS_ALLOWED_ACE *allow = (ACCESS_ALLOWED_ACE *)entry;
        const BYTE *sid_bytes = (const BYTE *)&allow->SidStart;
        size_t sid_length = 8 + 4 * (size_t)sid_bytes[1];
        if (sid_bytes[1] > SID_MAX_SUB_AUTHORITIES || sid_length > header->AceSize - offsetof(ACCESS_ALLOWED_ACE, SidStart)) { verdict = 4; break; }
        DWORD prohibited = GENERIC_ALL | GENERIC_WRITE | WRITE_DAC | WRITE_OWNER | DELETE | FILE_DELETE_CHILD | FILE_WRITE_DATA | FILE_APPEND_DATA | FILE_WRITE_EA | FILE_WRITE_ATTRIBUTES;
        if (private_directory) prohibited |= GENERIC_READ | GENERIC_EXECUTE | FILE_READ_DATA | FILE_READ_EA | FILE_READ_ATTRIBUTES | FILE_EXECUTE;
        if ((allow->Mask & prohibited) && !trusted_sid((PSID)&allow->SidStart, user->User.Sid, system.bytes, administrators.bytes)) { verdict = 3; break; }
    }
done:
    if (user) LocalFree(user);
    if (token) CloseHandle(token);
    if (descriptor) LocalFree(descriptor);
    return verdict;
}

#elif defined(__APPLE__) || defined(__linux__)
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>
#if defined(__APPLE__)
#include <sys/mount.h>
#include <sys/acl.h>
#include <fcntl.h>
#include <string.h>
#else
#include <sys/vfs.h>
#endif

int kairo_fs_guard(uintptr_t raw, int private_directory) {
    int fd = (int)raw;
    struct stat st;
    if (fstat(fd, &st) != 0) return 4;
    if (!S_ISDIR(st.st_mode)) return 6;
    if (st.st_uid != geteuid()) return 1;
    if (st.st_mode & (private_directory ? 0077 : 0022)) return 2;
#if defined(__APPLE__)
    struct statfs fs;
    if (fstatfs(fd, &fs) != 0) return 4;
    if (!(fs.f_flags & MNT_LOCAL) || strcmp(fs.f_fstypename, "apfs")) return 5;
    filesec_t security = filesec_init();
    if (!security) return 4;
    int present = -1;
    int result = fstatx_np(fd, &st, security);
    if (result == 0) result = filesec_query_property(security, FILESEC_ACL, &present);
    filesec_free(security);
    if (result != 0 || present < 0) return 4;
    /* An extended ACL needs separate interpretation; do not trust mode alone. */
    if (present) return 3;
#else
    struct statfs fs;
    if (fstatfs(fd, &fs) != 0) return 4;
    /* Local filesystems with POSIX DAC/ACL mask semantics. Fail closed for
       NFS/CIFS/FUSE and unknown permission models. */
    switch ((unsigned long)fs.f_type) {
        case 0xEF53UL: /* ext2/ext3/ext4 */
        case 0x58465342UL: /* XFS */
        case 0x9123683EUL: /* btrfs */
        case 0x01021994UL: /* tmpfs */
        case 0x794c7630UL: /* overlayfs */
            break;
        default: return 5;
    }
    /* POSIX named-user/group access is limited by the group mask represented
       in st_mode; the write mask above therefore also rejects ACL write grants. */
#endif
    return 0;
}
#else
int kairo_fs_guard(uintptr_t raw, int private_directory) {
    (void)raw; (void)private_directory; return 5;
}
#endif
