use crate::{
    device::block_dev::{BlockDevice, BlockDeviceError},
    fs::{
        FsError,
        vfs::{FileHandle, FileMetadata, FileSystem},
    },
    kernel::KERNEL,
};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};

use super::{
    boot_sector::BootSector,
    directory::RawDirEntry,
    fat_table::FatTable,
    file::FatFile,
    utils::{
        compute_lfn_checksum, create_lfn_entries, fill_8_3, generate_short_alias, is_dot_or_dot2,
        parse_dir, raw_idxes_for, same_entry,
    },
};

#[derive(Debug)]
pub struct Fat16FileSystem {
    id: usize,
    bs: BootSector,
    fat: FatTable,
    bytes_per_sector: u16,
    root_dir_sectors: u32,
    first_root_dir_sector: u32,
    first_data_sector: u32,
    total_clusters: u16,
    sectors_per_cluster: u8,
}

impl Fat16FileSystem {
    pub fn new(id: usize) -> Result<Self, BlockDeviceError> {
        let mut buffer: [u8; 512] = [0; 512];
        KERNEL.read_sectors(id, 0, 1, &mut buffer)?;
        let bs: BootSector = BootSector::read_from_buffer(&buffer)?;
        let bytes_per_sector = bs.bytes_per_sector;
        let root_dir_sectors = (bs.root_entry_count as u32 * 32).div_ceil(bytes_per_sector as u32);
        let first_fat_sector = bs.reserved_sectors as u32;
        let fat_size = bs.fat_size_16;
        let first_root_dir_sector = first_fat_sector + (bs.num_fats as u32 * fat_size as u32);
        let first_data_sector = first_root_dir_sector + root_dir_sectors;

        let total_sectors = if bs.total_sectors_16 != 0 {
            bs.total_sectors_16 as u32
        } else {
            bs.total_sectors_32
        };
        let data_sectors = total_sectors - first_data_sector;
        let total_clusters = (data_sectors / bs.sectors_per_cluster as u32) as u16;

        let fat = FatTable {
            first_fat_sector,
            sectors_per_fat: fat_size,
            num_fats: bs.num_fats,
        };
        Ok(Self {
            id,
            bs,
            fat,
            bytes_per_sector,
            root_dir_sectors,
            first_root_dir_sector,
            first_data_sector,
            total_clusters,
            sectors_per_cluster: bs.sectors_per_cluster,
        })
    }

    fn cluster_to_sector(&self, cluster: u16) -> u32 {
        if cluster < 2 {
            self.first_data_sector
        } else {
            self.first_data_sector + ((cluster as u32 - 2) * (self.sectors_per_cluster as u32))
        }
    }

    fn read_directory(&self, cluster: u16) -> Result<Vec<RawDirEntry>, BlockDeviceError> {
        let mut list = Vec::new();
        if cluster == 0 {
            for s in 0..self.root_dir_sectors {
                let lba = self.first_root_dir_sector + s;
                let mut buf = [0u8; 512];
                KERNEL.read_sectors(self.id, lba as u64, 1, &mut buf)?;
                for i in 0..16 {
                    let start = i * 32;
                    let raw: RawDirEntry = unsafe {
                        core::ptr::read(buf[start as usize..].as_ptr() as *const RawDirEntry)
                    };
                    list.push(raw);
                }
            }
        } else {
            let cluster_size = (self.sectors_per_cluster as usize) * 512;
            let mut cur = cluster;
            loop {
                if !(2..0xFFF8).contains(&cur) {
                    break;
                }
                let csec = self.cluster_to_sector(cur);
                let mut cbuf = vec![0u8; cluster_size];
                for s in 0..self.sectors_per_cluster {
                    let lba = csec + s as u32;
                    KERNEL.read_sectors(
                        self.id,
                        lba as u64,
                        1,
                        &mut cbuf[(s as usize) * 512..(s as usize) * 512 + 512],
                    )?;
                }
                let entries_count = cluster_size / 32;
                for i in 0..entries_count {
                    let start = i * 32;
                    let raw: RawDirEntry =
                        unsafe { core::ptr::read(cbuf[start..].as_ptr() as *const RawDirEntry) };
                    list.push(raw);
                }
                let next = self
                    .fat
                    .read_fat16_entry(self.id, cur, self.bytes_per_sector)?;
                cur = next;
            }
        }
        Ok(list)
    }

    fn write_directory(
        &self,
        cluster: u16,
        entries: &[RawDirEntry],
    ) -> Result<(), BlockDeviceError> {
        if cluster == 0 {
            let total_slots = (self.root_dir_sectors * (512 / 32)) as usize;
            if entries.len() > total_slots {
                return Err(BlockDeviceError::NoSpace);
            }
            let mut slice_idx = 0usize;
            for s in 0..self.root_dir_sectors {
                let lba = self.first_root_dir_sector + s;
                let mut buf = [0u8; 512];
                let mut to_write = 16;
                if slice_idx + 16 > entries.len() {
                    to_write = entries.len() - slice_idx;
                }
                for i in 0..to_write {
                    let raw = &entries[slice_idx + i];
                    let dst = i * 32;
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            raw as *const RawDirEntry as *const u8,
                            buf.as_mut_ptr().add(dst),
                            32,
                        );
                    }
                }
                KERNEL.write_sectors(self.id, lba as u64, 1, &buf)?;
                slice_idx += to_write;
                if slice_idx >= entries.len() {
                    break;
                }
            }
        } else {
            let cluster_size = (self.sectors_per_cluster as usize) * 512;
            let slots_per_cluster = cluster_size / 32;
            let needed_clusters = entries.len().div_ceil(slots_per_cluster);
            let mut chain = Vec::new();
            let mut cur = cluster;
            loop {
                chain.push(cur);
                let next = self
                    .fat
                    .read_fat16_entry(self.id, cur, self.bytes_per_sector)?;
                if !(2..0xFFF8).contains(&next) {
                    break;
                }
                cur = next;
            }
            while chain.len() < needed_clusters {
                let endc = *chain.last().unwrap();
                let newc = self.fat.alloc_cluster(
                    self.id,
                    self.bytes_per_sector,
                    2,
                    self.total_clusters,
                )?;
                self.fat
                    .extend_chain(self.id, endc, newc, self.bytes_per_sector)?;
                chain.push(newc);
            }

            let mut idx = 0usize;
            let mut remain = entries.len();
            for &c in chain.iter() {
                if remain == 0 {
                    break;
                }
                let mut cbuf = vec![0u8; cluster_size];
                let to_write = core::cmp::min(slots_per_cluster, remain);
                for i in 0..to_write {
                    let raw = &entries[idx + i];
                    let dst = i * 32;
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            raw as *const RawDirEntry as *const u8,
                            cbuf.as_mut_ptr().add(dst),
                            32,
                        );
                    }
                }
                let csec = self.cluster_to_sector(c);
                for s in 0..self.sectors_per_cluster {
                    let lba = csec + s as u32;
                    let start = (s as usize) * 512;
                    KERNEL.write_sectors(self.id, lba as u64, 1, &cbuf[start..start + 512])?;
                }
                idx += to_write;
                remain -= to_write;
            }
            if idx < entries.len() {
                return Err(BlockDeviceError::NoSpace);
            }
            if chain.len() > needed_clusters {
                for i in needed_clusters..chain.len() {
                    self.fat
                        .free_cluster_chain(self.id, chain[i], self.bytes_per_sector)?;
                }
                let cend = chain[needed_clusters - 1];
                self.fat
                    .write_fat16_entry(self.id, cend, 0xFFFF, self.bytes_per_sector)?;
            }
        }
        Ok(())
    }

    fn resolve_path(
        &self,
        path: &str,
    ) -> Result<(u16, Option<RawDirEntry>, String), BlockDeviceError> {
        let parts: Vec<&str> = path.split('/').filter(|x| !x.is_empty()).collect();
        let mut dir_cluster = 0u16;

        let mut idx = 0usize;
        while idx < parts.len() {
            let list = self.read_directory(dir_cluster)?;
            let parsed = parse_dir(&list);
            let part = parts[idx];
            let mut matched = None;
            for (lfn, short, raw) in parsed {
                if lfn.eq_ignore_ascii_case(part) || short.eq_ignore_ascii_case(part) {
                    matched = Some(raw);
                    break;
                }
            }
            if let Some(r) = matched {
                if idx == parts.len() - 1 {
                    return Ok((dir_cluster, Some(r), part.to_string()));
                } else {
                    if !r.is_dir() {
                        return Err(BlockDeviceError::NotFound);
                    }
                    let c = r.cluster_low;
                    dir_cluster = c;
                    idx += 1;
                }
            } else if idx == parts.len() - 1 {
                return Ok((dir_cluster, None, part.to_string()));
            } else {
                return Err(BlockDeviceError::NotFound);
            }
        }

        Ok((0, None, "".to_string()))
    }

    pub fn read_file_data(
        &self,
        start_cluster: u16,
        offset: usize,
        buf: &mut [u8],
        size: u32,
    ) -> Result<usize, BlockDeviceError> {
        if start_cluster < 2 || size == 0 {
            return Ok(0);
        }
        let cluster_size = (self.sectors_per_cluster as usize) * 512;
        let mut skip = offset;
        let remain = buf.len();
        let mut read_limit = (size as usize).saturating_sub(offset);
        if read_limit > remain {
            read_limit = remain;
        }
        if read_limit == 0 {
            return Ok(0);
        }

        let mut cur = start_cluster;
        let mut out_offset = 0usize;
        loop {
            if !(2..0xFFF8).contains(&cur) {
                break;
            }
            let csec = self.cluster_to_sector(cur) as u64;
            let mut cbuf = vec![0u8; cluster_size];
            for s in 0..self.sectors_per_cluster {
                KERNEL.read_sectors(
                    self.id,
                    csec + s as u64,
                    1,
                    &mut cbuf[s as usize * 512..s as usize * 512 + 512],
                )?;
            }
            if skip >= cluster_size {
                skip -= cluster_size;
            } else {
                let chunk = cluster_size - skip;
                let to_cp = core::cmp::min(chunk, read_limit);
                buf[out_offset..out_offset + to_cp].copy_from_slice(&cbuf[skip..skip + to_cp]);
                out_offset += to_cp;
                read_limit -= to_cp;
                skip = 0;
            }
            if read_limit == 0 {
                break;
            }
            let next = self
                .fat
                .read_fat16_entry(self.id, cur, self.bytes_per_sector)?;
            cur = next;
        }
        Ok(out_offset)
    }

    pub fn write_file_data(
        &self,
        start_cluster: &mut u16,
        size: &mut u32,
        offset: u32,
        data: &[u8],
    ) -> Result<usize, BlockDeviceError> {
        let cluster_size = (self.sectors_per_cluster as usize) * 512;
        let end_offset = offset as usize + data.len();
        let new_size = if end_offset > *size as usize {
            end_offset
        } else {
            *size as usize
        };

        if *start_cluster < 2 && !data.is_empty() {
            let c =
                self.fat
                    .alloc_cluster(self.id, self.bytes_per_sector, 2, self.total_clusters)?;
            *start_cluster = c;
        }
        let mut cur = *start_cluster;
        let mut skip = offset as usize;
        let mut data_offset = 0usize;
        if cur < 2 && data.is_empty() {
            return Ok(0);
        }

        loop {
            if skip < cluster_size {
                let csec = self.cluster_to_sector(cur) as u64;
                let mut cbuf = vec![0u8; cluster_size];
                if data.len() < cluster_size
                    || skip > 0
                    || offset as usize + data.len() < (cluster_size)
                {
                    for s in 0..self.sectors_per_cluster {
                        KERNEL.read_sectors(
                            self.id,
                            csec + s as u64,
                            1,
                            &mut cbuf[s as usize * 512..s as usize * 512 + 512],
                        )?;
                    }
                }
                let chunk = core::cmp::min(cluster_size - skip, data.len() - data_offset);
                cbuf[skip..skip + chunk].copy_from_slice(&data[data_offset..data_offset + chunk]);
                for s in 0..self.sectors_per_cluster {
                    KERNEL.write_sectors(
                        self.id,
                        csec + s as u64,
                        1,
                        &cbuf[s as usize * 512..s as usize * 512 + 512],
                    )?;
                }
                data_offset += chunk;
                skip = 0;
                if data_offset >= data.len() {
                    *size = new_size as u32;
                    return Ok(data.len());
                }
            } else {
                skip -= cluster_size;
            }
            let nxt = self
                .fat
                .read_fat16_entry(self.id, cur, self.bytes_per_sector)?;
            if !(2..0xFFF8).contains(&nxt) {
                let newc = self.fat.alloc_cluster(
                    self.id,
                    self.bytes_per_sector,
                    2,
                    self.total_clusters,
                )?;
                self.fat
                    .write_fat16_entry(self.id, cur, newc, self.bytes_per_sector)?;
                self.fat
                    .write_fat16_entry(self.id, newc, 0xFFFF, self.bytes_per_sector)?;
                cur = newc;
            } else {
                cur = nxt;
            }
        }
    }

    pub fn update_dir_entry_size(
        &self,
        loc: (u16, usize),
        cluster: u16,
        size: u32,
    ) -> Result<(), FsError> {
        let mut list = self.read_directory(loc.0).map_err(|_| FsError::IoError)?;
        if loc.1 >= list.len() {
            return Err(FsError::IoError);
        }
        list[loc.1].cluster_low = cluster;
        list[loc.1].file_size = size;
        self.write_directory(loc.0, &list)
            .map_err(|_| FsError::IoError)?;
        Ok(())
    }

    fn insert_entries(
        &self,
        dir_cluster: u16,
        entries: &[RawDirEntry],
    ) -> Result<usize, BlockDeviceError> {
        let mut list = self.read_directory(dir_cluster)?;
        let needed = entries.len();
        let mut start = None;
        let mut count = 0usize;
        for i in 0..list.len() {
            if list[i].is_free() {
                count += 1;
            } else {
                count = 0;
            }
            if count == needed {
                start = Some(i + 1 - needed);
                break;
            }
        }
        if start.is_none() && list.len() + needed <= 65536 {
            start = Some(list.len());
            for _ in 0..needed {
                let mut e = RawDirEntry {
                    name: [0x00; 8],
                    ext: [0x00; 3],
                    attr: 0,
                    nt_res: 0,
                    create_time_fine: 0,
                    create_time: 0,
                    create_date: 0,
                    access_date: 0,
                    cluster_high: 0,
                    mod_time: 0,
                    mod_date: 0,
                    cluster_low: 0,
                    file_size: 0,
                };
                e.name[0] = 0xE5;
                list.push(e);
            }
        }
        if let Some(pos) = start {
            for i in 0..needed {
                list[pos + i] = entries[i];
            }
            self.write_directory(dir_cluster, &list)?;
            Ok(pos + needed - 1)
        } else {
            Err(BlockDeviceError::NoSpace)
        }
    }

    fn init_dir_cluster(&self, cluster: u16, parent: u16) -> Result<(), BlockDeviceError> {
        let cluster_size = (self.sectors_per_cluster as usize) * 512;
        let csec = self.cluster_to_sector(cluster) as u64;
        let mut cbuf = vec![0u8; cluster_size];
        let dot = RawDirEntry {
            name: [b'.', 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20],
            ext: [0x20; 3],
            attr: 0x10,
            nt_res: 0,
            create_time_fine: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: 0,
            mod_time: 0,
            mod_date: 0,
            cluster_low: cluster,
            file_size: 0,
        };
        let dot2 = RawDirEntry {
            name: [b'.', b'.', 0x20, 0x20, 0x20, 0x20, 0x20, 0x20],
            ext: [0x20; 3],
            attr: 0x10,
            nt_res: 0,
            create_time_fine: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: 0,
            mod_time: 0,
            mod_date: 0,
            cluster_low: parent,
            file_size: 0,
        };
        unsafe {
            core::ptr::copy_nonoverlapping(
                &dot as *const RawDirEntry as *const u8,
                cbuf.as_mut_ptr(),
                32,
            );
            core::ptr::copy_nonoverlapping(
                &dot2 as *const RawDirEntry as *const u8,
                cbuf.as_mut_ptr().add(32),
                32,
            );
        }
        for s in 0..self.sectors_per_cluster {
            KERNEL.write_sectors(
                self.id,
                csec + s as u64,
                1,
                &cbuf[s as usize * 512..s as usize * 512 + 512],
            )?;
        }
        Ok(())
    }
}

impl FileSystem for Fat16FileSystem {
    fn open(&self, path: &str) -> Result<FileHandle, FsError> {
        let (dir_cluster, entry_opt, _) = self.resolve_path(path).map_err(|_| FsError::IoError)?;
        if entry_opt.is_none() {
            return Err(FsError::NotFound);
        }
        let entry = entry_opt.unwrap();
        if entry.is_dir() {
            return Err(FsError::IsADirectory);
        }

        let cluster = entry.cluster_low;
        let size = entry.file_size;
        let list = self
            .read_directory(dir_cluster)
            .map_err(|_| FsError::IoError)?;
        let parsed = parse_dir(&list);
        let mut foundi = None;
        for (i, (_lfn, _s, raw)) in parsed.iter().enumerate() {
            if same_entry(raw, &entry) {
                foundi = Some(i);
                break;
            }
        }
        if foundi.is_none() {
            return Err(FsError::NotFound);
        }
        let real_idx = foundi.unwrap();

        let f = FatFile::new(
            Arc::new(self.clone()),
            cluster,
            size,
            (dir_cluster, real_idx),
        );

        Ok(FileHandle::new(Box::new(f)))
    }

    fn read_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let (dir_cluster, entry_opt, final_name) =
            self.resolve_path(path).map_err(|_| FsError::IoError)?;
        if path == "/" || (dir_cluster == 0 && final_name.is_empty()) {
            let list = self.read_directory(0).map_err(|_| FsError::IoError)?;
            let parsed = parse_dir(&list);
            let mut out = Vec::new();
            for (lfn, short, raw) in parsed {
                if raw.is_free() {
                    continue;
                }
                if !lfn.is_empty() {
                    out.push(lfn);
                } else {
                    out.push(short);
                }
            }
            return Ok(out);
        }
        if let Some(e) = entry_opt {
            if !e.is_dir() {
                return Err(FsError::IsADirectory);
            }
            let c = e.cluster_low;
            let list = self.read_directory(c).map_err(|_| FsError::IoError)?;
            let parsed = parse_dir(&list);
            let mut out = Vec::new();
            for (lfn, short, raw) in parsed {
                if raw.is_free() {
                    continue;
                }
                if !lfn.is_empty() {
                    out.push(lfn);
                } else {
                    out.push(short);
                }
            }
            Ok(out)
        } else {
            Err(FsError::NotFound)
        }
    }

    fn create(&self, path: &str, directory: bool) -> Result<(), FsError> {
        let (dir_cluster, entry_opt, final_name) =
            self.resolve_path(path).map_err(|_| FsError::IoError)?;
        if final_name.is_empty() && dir_cluster == 0 && !directory {
            return Err(FsError::InvalidPath);
        }
        if entry_opt.is_some() {
            return Err(FsError::AlreadyExists);
        }
        let short_alias = generate_short_alias(&final_name);
        let mut short_raw = RawDirEntry {
            name: [0x20; 8],
            ext: [0x20; 3],
            attr: if directory { 0x10 } else { 0x20 },
            nt_res: 0,
            create_time_fine: 0,
            create_time: 0,
            create_date: 0,
            access_date: 0,
            cluster_high: 0,
            mod_time: 0,
            mod_date: 0,
            cluster_low: 0,
            file_size: 0,
        };
        fill_8_3(&mut short_raw, &short_alias);
        if directory {
            let c = self
                .fat
                .alloc_cluster(self.id, self.bytes_per_sector, 2, self.total_clusters)
                .map_err(|_| FsError::NoSpace)?;
            self.fat
                .write_fat16_entry(self.id, c, 0xFFFF, self.bytes_per_sector)
                .map_err(|_| FsError::IoError)?;
            short_raw.cluster_low = c;
            self.init_dir_cluster(c, if dir_cluster == 0 { c } else { dir_cluster })
                .map_err(|_| FsError::IoError)?;
        }

        let fits_8_3 = final_name.len() <= 12 && final_name.find('.').is_none_or(|idx| idx <= 8);
        if fits_8_3 {
            let _ = self
                .insert_entries(dir_cluster, &[short_raw])
                .map_err(|_| FsError::NoSpace)?;
            return Ok(());
        } else {
            let chksum = compute_lfn_checksum(&short_raw);
            let lfn_entries = create_lfn_entries(&final_name, chksum);
            let mut all = Vec::new();
            all.extend_from_slice(&lfn_entries);
            all.push(short_raw);
            self.insert_entries(dir_cluster, &all)
                .map_err(|_| FsError::NoSpace)?;
        }
        Ok(())
    }

    fn remove(&self, path: &str) -> Result<(), FsError> {
        let (dir_cluster, entry_opt, _) = self.resolve_path(path).map_err(|_| FsError::IoError)?;
        if entry_opt.is_none() {
            return Err(FsError::NotFound);
        }
        let e = entry_opt.unwrap();
        if e.is_dir() {
            let c = e.cluster_low;
            if c > 1 {
                let list = self.read_directory(c).map_err(|_| FsError::IoError)?;
                if list
                    .iter()
                    .any(|x| !x.is_free() && !x.is_lfn() && !is_dot_or_dot2(x))
                {
                    return Err(FsError::PermissionDenied);
                }
            }
            self.fat
                .free_cluster_chain(self.id, c, self.bytes_per_sector)
                .map_err(|_| FsError::IoError)?;
        } else if e.cluster_low >= 2 {
            self.fat
                .free_cluster_chain(self.id, e.cluster_low, self.bytes_per_sector)
                .map_err(|_| FsError::IoError)?;
        }
        let mut list = self
            .read_directory(dir_cluster)
            .map_err(|_| FsError::IoError)?;
        let parsed = parse_dir(&list);
        let mut final_idx = None;
        for (i, (_lfn, _s, raw)) in parsed.iter().enumerate() {
            if same_entry(raw, &e) {
                final_idx = Some(i);
                break;
            }
        }
        if final_idx.is_none() {
            return Err(FsError::NotFound);
        }
        let fi = final_idx.unwrap();
        let mut lfn_count = 0usize;
        let mut tmp = fi;
        while tmp > 0 {
            let r = parsed[tmp - 1].2;
            if r.is_lfn() {
                lfn_count += 1;
                tmp -= 1;
            } else {
                break;
            }
        }
        let needed = lfn_count + 1;
        let mut raw_idxes = Vec::new();
        let full_parsed = parse_dir(&list);
        for (idx, (_, _, raw)) in full_parsed.iter().enumerate() {
            if idx >= tmp && idx < tmp + needed {
                raw_idxes_for(&list, raw, &mut raw_idxes);
            }
        }
        for ri in &raw_idxes {
            list[*ri].name[0] = 0xE5;
        }
        self.write_directory(dir_cluster, &list)
            .map_err(|_| FsError::IoError)?;
        Ok(())
    }

    fn metadata(&self, path: &str) -> Result<FileMetadata, FsError> {
        let (_dir, eopt, _fname) = self.resolve_path(path).map_err(|_| FsError::IoError)?;
        if eopt.is_none() {
            return Err(FsError::NotFound);
        }
        let e = eopt.unwrap();
        let size = e.file_size as u64;
        let mode = if e.is_dir() { 0o755 } else { 0o644 };
        Ok(FileMetadata {
            uid: 0,
            gid: 0,
            mode,
            size,
            is_dir: e.is_dir(),
        })
    }

    fn chmod(&self, _path: &str, _mode: u16) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }

    fn chown(&self, _path: &str, _uid: u32, _gid: u32) -> Result<(), FsError> {
        Err(FsError::Unsupported)
    }
}

impl Clone for Fat16FileSystem {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            bs: self.bs,
            fat: FatTable {
                first_fat_sector: self.fat.first_fat_sector,
                sectors_per_fat: self.fat.sectors_per_fat,
                num_fats: self.fat.num_fats,
            },
            bytes_per_sector: self.bytes_per_sector,
            root_dir_sectors: self.root_dir_sectors,
            first_root_dir_sector: self.first_root_dir_sector,
            first_data_sector: self.first_data_sector,
            total_clusters: self.total_clusters,
            sectors_per_cluster: self.sectors_per_cluster,
        }
    }
}
