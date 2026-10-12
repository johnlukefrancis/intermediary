// Path: src-tauri/src/lib/launch/shell_link.rs
// Description: Minimal MS-SHLLINK reader that recovers a shortcut's target path from its LinkInfo

const HEADER_SIZE: usize = 0x4C;
const HAS_ID_LIST: u32 = 0x1;
const HAS_LINK_INFO: u32 = 0x2;
const FORCE_NO_LINK_INFO: u32 = 0x100;
const VOLUME_AND_LOCAL_BASE: u32 = 0x1;
const NETWORK_AND_SUFFIX: u32 = 0x2;

/// The target path a `.lnk` names, or `None` when it has no LinkInfo or is malformed.
pub fn link_target(bytes: &[u8]) -> Option<String> {
    if read_u32(bytes, 0)? as usize != HEADER_SIZE {
        return None;
    }
    let flags = read_u32(bytes, 20)?;
    let mut offset = HEADER_SIZE;
    if flags & HAS_ID_LIST != 0 {
        offset += 2 + read_u16(bytes, offset)? as usize;
    }
    if flags & HAS_LINK_INFO == 0 || flags & FORCE_NO_LINK_INFO != 0 {
        return None;
    }
    let info_size = read_u32(bytes, offset)? as usize;
    let info = bytes.get(offset..offset.checked_add(info_size)?)?;
    link_info_target(info)
}

fn link_info_target(info: &[u8]) -> Option<String> {
    let header_size = read_u32(info, 4)?;
    let info_flags = read_u32(info, 8)?;
    let has_unicode = header_size >= 0x24;
    let suffix = if has_unicode {
        utf16_at(info, read_u32(info, 32)? as usize)
    } else {
        ansi_at(info, read_u32(info, 24)? as usize)
    }
    .unwrap_or_default();

    if info_flags & VOLUME_AND_LOCAL_BASE != 0 {
        let base = if has_unicode {
            utf16_at(info, read_u32(info, 28)? as usize)
        } else {
            ansi_at(info, read_u32(info, 16)? as usize)
        }?;
        return Some(join(&base, &suffix));
    }
    if info_flags & NETWORK_AND_SUFFIX != 0 {
        let network = info.get(read_u32(info, 20)? as usize..)?;
        let net_name_offset = read_u32(network, 8)? as usize;
        let net_name = if net_name_offset > 0x14 {
            utf16_at(network, read_u32(network, 20)? as usize)
        } else {
            ansi_at(network, net_name_offset)
        }?;
        return Some(join(&net_name, &suffix));
    }
    None
}

fn join(base: &str, suffix: &str) -> String {
    if suffix.is_empty() {
        base.to_string()
    } else {
        format!("{}\\{}", base.trim_end_matches('\\'), suffix)
    }
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn ansi_at(bytes: &[u8], at: usize) -> Option<String> {
    let tail = bytes.get(at..)?;
    let end = tail.iter().position(|b| *b == 0)?;
    Some(tail[..end].iter().map(|b| char::from(*b)).collect())
}

fn utf16_at(bytes: &[u8], at: usize) -> Option<String> {
    let tail = bytes.get(at..)?;
    let units: Vec<u16> = tail
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .take_while(|unit| *unit != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

#[cfg(test)]
mod tests {
    use super::link_target;

    fn header(flags: u32) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x4C];
        bytes[0..4].copy_from_slice(&0x4Cu32.to_le_bytes());
        bytes[20..24].copy_from_slice(&flags.to_le_bytes());
        bytes
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn reads_local_base_path_after_id_list() {
        let mut bytes = header(0x1 | 0x2);
        bytes.extend_from_slice(&3u16.to_le_bytes());
        bytes.extend_from_slice(&[9, 9, 9]);
        let path = b"E:\\UnrealProjects\\Blorb\\Blorb.uproject\0";
        let mut info = Vec::new();
        let header_size = 0x1Cu32;
        let local_offset = header_size;
        let suffix_offset = local_offset + path.len() as u32;
        push_u32(&mut info, suffix_offset + 1);
        push_u32(&mut info, header_size);
        push_u32(&mut info, 0x1);
        push_u32(&mut info, 0);
        push_u32(&mut info, local_offset);
        push_u32(&mut info, 0);
        push_u32(&mut info, suffix_offset);
        info.extend_from_slice(path);
        info.push(0);
        bytes.extend_from_slice(&info);
        assert_eq!(
            link_target(&bytes).as_deref(),
            Some(r"E:\UnrealProjects\Blorb\Blorb.uproject")
        );
    }

    #[test]
    fn joins_network_name_and_suffix() {
        let mut bytes = header(0x2);
        let net = b"\\\\WSL.LOCALHOST\\UBUNTU\0";
        let suffix = b"home\\johnf\\dev\\Coilbound\\Coilbound.html\0";
        let header_size = 0x1Cu32;
        let network_offset = header_size;
        let network_len = 0x14 + net.len() as u32;
        let suffix_offset = network_offset + network_len;
        let mut info = Vec::new();
        push_u32(&mut info, suffix_offset + suffix.len() as u32);
        push_u32(&mut info, header_size);
        push_u32(&mut info, 0x2);
        push_u32(&mut info, 0);
        push_u32(&mut info, 0);
        push_u32(&mut info, network_offset);
        push_u32(&mut info, suffix_offset);
        push_u32(&mut info, network_len);
        push_u32(&mut info, 0);
        push_u32(&mut info, 0x14);
        push_u32(&mut info, 0);
        push_u32(&mut info, 0);
        info.extend_from_slice(net);
        info.extend_from_slice(suffix);
        bytes.extend_from_slice(&info);
        assert_eq!(
            link_target(&bytes).as_deref(),
            Some(r"\\WSL.LOCALHOST\UBUNTU\home\johnf\dev\Coilbound\Coilbound.html")
        );
    }

    #[test]
    fn rejects_truncated_input() {
        assert_eq!(link_target(&header(0x2)[..40]), None);
        assert_eq!(link_target(&header(0x2)), None);
    }
}
