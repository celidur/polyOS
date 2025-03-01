use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

use super::directory::RawDirEntry;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct LfnEntry {
    pub order: u8,
    pub name1: [u16; 5],
    pub attr: u8,
    pub lfn_type: u8,
    pub checksum: u8,
    pub name2: [u16; 6],
    pub cluster_low: u16,
    pub name3: [u16; 2],
}

pub fn short_name_to_string(raw: &RawDirEntry) -> String {
    let mut b = String::new();
    for c in &raw.name {
        if *c == 0x20 {
            break;
        }
        b.push(*c as char);
    }
    let mut e = String::new();
    for c in &raw.ext {
        if *c == 0x20 {
            break;
        }
        e.push(*c as char);
    }
    if e.is_empty() {
        b
    } else {
        format!("{}.{}", b, e)
    }
}

pub fn is_dot_or_dot2(e: &RawDirEntry) -> bool {
    if e.name[0] == b'.' && e.name[1] == 0x20 {
        return true; // "."
    }
    if e.name[0] == b'.' && e.name[1] == b'.' {
        return true; // ".."
    }
    false
}

pub fn same_entry(a: &RawDirEntry, b: &RawDirEntry) -> bool {
    if a.file_size != b.file_size {
        return false;
    }
    if a.cluster_low != b.cluster_low {
        return false;
    }
    for i in 0..8 {
        if a.name[i] != b.name[i] {
            return false;
        }
    }
    for i in 0..3 {
        if a.ext[i] != b.ext[i] {
            return false;
        }
    }
    true
}

pub fn raw_idxes_for(list: &[RawDirEntry], raw: &RawDirEntry, out: &mut Vec<usize>) {
    let mut i = 0;
    while i < list.len() {
        if same_entry(&list[i], raw) {
            out.push(i);
            let mut j = i;
            while j > 0 {
                if list[j - 1].is_lfn() {
                    out.push(j - 1);
                    j -= 1;
                } else {
                    break;
                }
            }
            break;
        }
        i += 1;
    }
}

pub fn fat_offset(cluster: u16) -> u32 {
    (cluster as u32) * 2
}

fn read_lfn_parts(l: &LfnEntry) -> Vec<u16> {
    let mut out = Vec::new();
    out.extend(l.name1);
    out.extend(l.name2);
    out.extend(l.name3);
    out
}

fn ucs2_to_string(units: &[u16]) -> String {
    let mut s = String::new();
    for &ch in units {
        if ch == 0x0000 || ch == 0xFFFF {
            break;
        }
        s.push(ch as u8 as char);
    }
    s
}

fn build_long_name(entries: &[LfnEntry]) -> String {
    let mut v = entries.to_owned();
    v.sort_by_key(|x| x.order & 0x3F);
    let mut out = String::new();
    for l in &v {
        let all = read_lfn_parts(l);
        let part = ucs2_to_string(&all);
        out.push_str(&part);
    }
    out
}

pub fn parse_dir(raw_list: &[RawDirEntry]) -> Vec<(String, String, RawDirEntry)> {
    let mut result = Vec::new();
    let mut lfn_temp = Vec::new();
    for r in raw_list {
        if r.is_free() {
            lfn_temp.clear();
            continue;
        }
        if r.is_lfn() {
            let lfn: LfnEntry = unsafe { core::mem::transmute(*r) };
            lfn_temp.push(lfn);
        } else {
            let short = short_name_to_string(r);
            let lfn_name = if !lfn_temp.is_empty() {
                let s = build_long_name(&lfn_temp);
                lfn_temp.clear();
                s
            } else {
                String::new()
            };
            result.push((lfn_name, short, *r));
        }
    }
    result
}

pub fn compute_lfn_checksum(short: &RawDirEntry) -> u8 {
    let mut sum = 0u8;
    for i in 0..11 {
        sum = ((sum & 1) << 7)
            .wrapping_add(sum >> 1)
            .wrapping_add(short.name_and_ext_byte(i));
    }
    sum
}

pub fn create_lfn_entries(fullname: &str, checksum: u8) -> Vec<RawDirEntry> {
    let mut unicode: Vec<u16> = fullname.encode_utf16().collect();
    let mut entries = Vec::new();

    let mut seq = 0u8;
    while !unicode.is_empty() {
        seq += 1;
        let chunk = &unicode[..core::cmp::min(13, unicode.len())];
        let left = &unicode[core::cmp::min(13, unicode.len())..];
        let mut lfn: LfnEntry = LfnEntry {
            order: seq,
            name1: [0u16; 5],
            attr: 0x0F,
            lfn_type: 0,
            checksum,
            name2: [0u16; 6],
            cluster_low: 0,
            name3: [0u16; 2],
        };
        let mut cpos = 0;
        for i in 0..5 {
            if cpos < chunk.len() {
                lfn.name1[i] = chunk[cpos];
                cpos += 1;
            } else if i == 0 {
                lfn.name1[i] = 0x0000;
            } else {
                lfn.name1[i] = 0xFFFF;
            }
        }
        for i in 0..6 {
            if cpos < chunk.len() {
                lfn.name2[i] = chunk[cpos];
                cpos += 1;
            } else if i == 0 && cpos == 5 {
                lfn.name2[i] = 0x0000;
            } else {
                lfn.name2[i] = 0xFFFF;
            }
        }
        for i in 0..2 {
            if cpos < chunk.len() {
                lfn.name3[i] = chunk[cpos];
                cpos += 1;
            } else if i == 0 {
                lfn.name3[i] = 0x0000;
            } else {
                lfn.name3[i] = 0xFFFF;
            }
        }
        entries.push(unsafe { core::mem::transmute::<LfnEntry, RawDirEntry>(lfn) });
        unicode = left.to_vec();
    }

    if let Some(last) = entries.last_mut() {
        let lfn: &mut LfnEntry =
            unsafe { core::mem::transmute::<&mut RawDirEntry, &mut LfnEntry>(last) };
        lfn.order |= 0x40;
    }

    entries.reverse();
    entries
}

pub fn generate_short_alias(name: &str) -> String {
    let mut base;
    let mut ext = String::new();

    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() == 1 {
        base = parts[0].to_uppercase();
    } else {
        ext = parts.last().unwrap().to_uppercase();
        base = parts[..parts.len() - 1].join(".").to_uppercase();
    }
    let invalid = ":/\\\"*+<>?;[]|=";
    base = base.chars().filter(|c| !invalid.contains(*c)).collect();
    ext = ext.chars().filter(|c| !invalid.contains(*c)).collect();

    if base.len() > 8 {
        base.truncate(6);
    }
    if ext.len() > 3 {
        ext.truncate(3);
    }

    if base.len() <= 8 && ext.len() <= 3 {
        let short = if ext.is_empty() {
            base.clone()
        } else {
            format!("{}.{}", base, ext)
        };
        if short.len() <= 12 {
            // might check existence
            return short;
        }
    }

    if base.len() > 6 {
        base.truncate(6);
    }
    base.push('~');
    base.push('1');
    if !ext.is_empty() {
        base.push('.');
        base.push_str(&ext);
    }
    base
}

pub fn fill_8_3(raw: &mut RawDirEntry, short: &str) {
    let (b, e) = if let Some(idx) = short.find('.') {
        (short[..idx].to_owned(), short[idx + 1..].to_owned())
    } else {
        (short.to_owned(), "".to_owned())
    };
    let b_up = b.to_uppercase();
    let e_up = e.to_uppercase();

    for i in 0..8 {
        raw.name[i] = if i < b_up.len() {
            b_up.as_bytes()[i]
        } else {
            0x20
        };
    }
    for i in 0..3 {
        raw.ext[i] = if i < e_up.len() {
            e_up.as_bytes()[i]
        } else {
            0x20
        };
    }
}
