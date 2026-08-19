use crate::cpu::memory;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const DESC_NEXT: u16 = 1;
const DESC_WRITE: u16 = 2;
const VIRTIO_9P_COMMON: i32 = 0xA800;
const VIRTIO_9P_NOTIFY: i32 = 0xA900;
const VIRTIO_9P_ISR: i32 = 0xA700;
const VIRTIO_9P_CONFIG: i32 = 0xA600;
const PCI_CONFIG_ADDRESS: i32 = 0xCF8;
const PCI_CONFIG_DATA: i32 = 0xCFC;

#[derive(Clone, Default)]
struct Queue {
    size: u16,
    enabled: bool,
    desc: u32,
    avail: u32,
    avail_last: u16,
    used: u32,
    staged: u16,
}

#[derive(Clone, Default)]
struct Fid {
    path: PathBuf,
    opened: bool,
}

struct Virtio9p {
    queue: Queue,
    status: u8,
    isr: u8,
    tag: Vec<u8>,
    root: Option<PathBuf>,
    fids: HashMap<u32, Fid>,
}

impl Default for Virtio9p {
    fn default() -> Self {
        Self {
            queue: Queue {
                size: 32,
                ..Queue::default()
            },
            status: 0,
            isr: 0,
            tag: b"host9p".to_vec(),
            root: None,
            fids: HashMap::new(),
        }
    }
}

#[derive(Default)]
struct DeviceBus {
    ninep: Virtio9p,
    pci_address: u32,
}
static BUS: OnceLock<Mutex<DeviceBus>> = OnceLock::new();

fn bus() -> &'static Mutex<DeviceBus> {
    BUS.get_or_init(|| {
        Mutex::new(DeviceBus {
            ninep: Virtio9p::default(),
            pci_address: 0,
        })
    })
}

pub fn set_9p_root(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path
        .as_ref()
        .canonicalize()
        .map_err(|e| format!("9p root {}: {e}", path.as_ref().display()))?;
    let mut b = bus().lock().map_err(|_| "device bus poisoned".to_owned())?;
    b.ninep.root = Some(path.clone());
    b.ninep.fids.insert(
        0,
        Fid {
            path,
            opened: false,
        },
    );
    Ok(())
}

pub fn restore_state(state: &serde_json::Value, buffers: &[Vec<u8>]) -> Result<(), String> {
    let slots = state.as_array().ok_or("v86 state is not an array")?;
    let mut b = bus().lock().map_err(|_| "device bus poisoned".to_owned())?;
    let Some(s) = slots.get(45).and_then(|v| v.as_array()) else {
        return Ok(());
    };
    if let Some(tag) = s.get(0).and_then(|v| v.as_array()) {
        b.ninep.tag = tag
            .iter()
            .filter_map(|v| v.as_u64().map(|x| x as u8))
            .collect();
    }
    if let Some(v) = s.get(2).and_then(|v| v.as_array()) {
        if let Some(q) = v.get(10).and_then(|v| v.as_array()) {
            b.ninep.queue.size = q.first().and_then(|v| v.as_u64()).unwrap_or(32) as u16;
            b.ninep.queue.enabled = q.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
            b.ninep.queue.desc = q.get(4).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            b.ninep.queue.avail = q.get(5).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            b.ninep.queue.avail_last = q.get(6).and_then(|v| v.as_u64()).unwrap_or(0) as u16;
            b.ninep.queue.used = q.get(7).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        }
    }
    b.ninep.fids.clear();
    if let Some(fids) = s.get(8).and_then(|v| v.as_array()) {
        for (id, fid) in fids.iter().enumerate() {
            let inode = fid.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
            b.ninep.fids.insert(
                id as u32,
                Fid {
                    path: PathBuf::from(format!("inode-{inode}")),
                    opened: false,
                },
            );
        }
    }
    let _ = buffers;
    Ok(())
}

pub fn io_read8(port: i32) -> Option<i32> {
    let b = bus().lock().ok()?;
    if (VIRTIO_9P_ISR..VIRTIO_9P_ISR + 4).contains(&port) {
        return Some(b.ninep.isr as i32);
    }
    if (VIRTIO_9P_CONFIG..VIRTIO_9P_CONFIG + 8).contains(&port) {
        let off = port - VIRTIO_9P_CONFIG;
        return Some(if off < b.ninep.tag.len() as i32 {
            b.ninep.tag[off as usize] as i32
        } else {
            0
        });
    }
    None
}

pub fn mmio_read8(addr: u32) -> Option<i32> {
    io_read8(addr as i32)
}

pub fn mmio_read32(addr: u32) -> Option<i32> {
    io_read32(addr as i32)
}

pub fn mmio_write8(addr: u32, value: i32) -> bool {
    io_write8(addr as i32, value)
}

pub fn mmio_write16(addr: u32, value: i32) -> bool {
    io_write16(addr as i32, value)
}

pub fn mmio_write32(addr: u32, value: i32) -> bool {
    io_write32(addr as i32, value)
}

pub fn io_read32(port: i32) -> Option<i32> {
    let b = bus().lock().ok()?;
    if port == PCI_CONFIG_DATA {
        return Some(pci_config_read(b.pci_address) as i32);
    }
    if (VIRTIO_9P_COMMON..VIRTIO_9P_COMMON + 0x100).contains(&port) {
        let off = port - VIRTIO_9P_COMMON;
        return Some(match off {
            0 => 0,
            4 => 0,
            8 => 0,
            12 => 0,
            20 => b.ninep.status as i32,
            24 => 1,
            _ => 0,
        });
    }
    None
}

pub fn io_write8(port: i32, value: i32) -> bool {
    if port == VIRTIO_9P_ISR {
        if let Ok(mut b) = bus().lock() {
            b.ninep.isr = 0;
        }
        return true;
    }
    if (VIRTIO_9P_COMMON..VIRTIO_9P_COMMON + 0x100).contains(&port) {
        if let Ok(mut b) = bus().lock() {
            if port - VIRTIO_9P_COMMON == 20 {
                b.ninep.status = value as u8;
            }
        }
        return true;
    }
    false
}

pub fn io_write16(port: i32, _value: i32) -> bool {
    if port == VIRTIO_9P_NOTIFY {
        process_queue();
        return true;
    }
    false
}

pub fn io_write32(port: i32, value: i32) -> bool {
    if let Ok(mut b) = bus().lock() {
        if port == PCI_CONFIG_ADDRESS {
            b.pci_address = value as u32;
            return true;
        }
    }
    io_write16(port, 0)
}

fn pci_config_read(address: u32) -> u32 {
    if address & 0x8000_0000 == 0 || (address >> 11) & 0x1F != 0 {
        return 0xFFFF_FFFF;
    }
    match (address >> 2) & 0x3F {
        0 => 0x1009_1AF4,
        2 => 0x0180_0000,
        4 => 0x0000_A001,
        8 => 0xFF00_0000,
        _ => 0,
    }
}

fn read8(addr: u32) -> u8 {
    unsafe { memory::read8_no_mmap_check(addr) as u8 }
}
fn read16(addr: u32) -> u16 {
    unsafe { memory::read16_no_mmap_check(addr) as u16 }
}
fn read32(addr: u32) -> u32 {
    unsafe { memory::read32_no_mmap_check(addr) as u32 }
}
fn write16(addr: u32, value: u16) {
    unsafe { memory::write16_no_mmap_or_dirty_check(addr, value as i32) }
}
fn write32(addr: u32, value: u32) {
    unsafe { memory::write32_no_mmap_or_dirty_check(addr, value as i32) }
}

fn process_queue() {
    let mut b = match bus().lock() {
        Ok(x) => x,
        Err(_) => return,
    };
    let mut q = b.ninep.queue.clone();
    if !q.enabled || q.desc == 0 || q.avail == 0 || q.used == 0 {
        return;
    }
    let avail_idx = read16(q.avail + 2);
    while q.avail_last != avail_idx {
        let head = read16(q.avail + 4 + 2u32 * (q.avail_last & q.size.saturating_sub(1)) as u32);
        let mut request = Vec::new();
        let mut writable = Vec::new();
        let mut idx = head;
        for _ in 0..q.size {
            let p = q.desc + idx as u32 * 16;
            let addr = read32(p);
            let len = read32(p + 8);
            let flags = read16(p + 12);
            let next = read16(p + 14);
            let mut buf = vec![0u8; len as usize];
            for (i, x) in buf.iter_mut().enumerate() {
                *x = read8(addr + i as u32);
            }
            if flags & DESC_WRITE != 0 {
                writable.push((addr, len));
            } else {
                request.extend_from_slice(&buf);
            }
            if flags & DESC_NEXT == 0 {
                break;
            }
            idx = next;
        }
        let reply = handle_9p(&mut b.ninep, &request);
        let mut pos = 0usize;
        let mut written = 0u32;
        for (addr, len) in writable {
            let n = (len as usize).min(reply.len().saturating_sub(pos));
            for i in 0..n {
                unsafe {
                    memory::write8_no_mmap_or_dirty_check(addr + i as u32, reply[pos + i] as i32);
                }
            }
            pos += n;
            written += n as u32;
        }
        let used_idx = read16(q.used + 2);
        let ring_off = 8u32 * (used_idx & (q.size - 1)) as u32;
        write32(q.used + 4 + ring_off, head as u32);
        write32(q.used + 8 + ring_off, written);
        write16(q.used + 2, used_idx.wrapping_add(1));
        q.avail_last = q.avail_last.wrapping_add(1);
        b.ninep.isr |= 1;
    }
    b.ninep.queue.avail_last = q.avail_last;
}

fn u16_at(x: &[u8], p: &mut usize) -> u16 {
    let v = u16::from_le_bytes([x[*p], x[*p + 1]]);
    *p += 2;
    v
}
fn u32_at(x: &[u8], p: &mut usize) -> u32 {
    let v = u32::from_le_bytes([x[*p], x[*p + 1], x[*p + 2], x[*p + 3]]);
    *p += 4;
    v
}
fn string_at(x: &[u8], p: &mut usize) -> String {
    let n = u16_at(x, p) as usize;
    let e = (*p + n).min(x.len());
    let s = String::from_utf8_lossy(&x[*p..e]).into_owned();
    *p = e;
    s
}
fn reply(id: u8, tag: u16, payload: &[u8]) -> Vec<u8> {
    let mut r = Vec::with_capacity(payload.len() + 7);
    r.extend_from_slice(&((payload.len() + 7) as u32).to_le_bytes());
    r.push(id + 1);
    r.extend_from_slice(&tag.to_le_bytes());
    r.extend_from_slice(payload);
    r
}
fn handle_9p(dev: &mut Virtio9p, req: &[u8]) -> Vec<u8> {
    if req.len() < 7 {
        return reply(6, 0, &2u32.to_le_bytes());
    }
    let mut p = 4;
    let id = req[p];
    p += 1;
    let tag = u16_at(req, &mut p);
    match id {
        100 => {
            let m = u32_at(req, &mut p);
            let _version = string_at(req, &mut p);
            let mut out = Vec::new();
            out.extend_from_slice(&m.min(8192).to_le_bytes());
            out.extend_from_slice(&6u16.to_le_bytes());
            out.extend_from_slice(b"9P2000.L");
            reply(id, tag, &out)
        }
        104 => {
            let fid = u32_at(req, &mut p);
            let _afid = u32_at(req, &mut p);
            let _uname = string_at(req, &mut p);
            let _aname = string_at(req, &mut p);
            let root = dev.root.clone().unwrap_or_else(|| PathBuf::from("."));
            dev.fids.insert(
                fid,
                Fid {
                    path: root,
                    opened: false,
                },
            );
            let mut out = vec![0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            reply(id, tag, &out)
        }
        110 => {
            let fid = u32_at(req, &mut p);
            let newfid = u32_at(req, &mut p);
            let nwname = u16_at(req, &mut p);
            let base = dev
                .fids
                .get(&fid)
                .map(|f| f.path.clone())
                .unwrap_or_default();
            let mut path = base;
            let mut qids = Vec::new();
            for _ in 0..nwname {
                let name = string_at(req, &mut p);
                path.push(&name);
                if !path.exists() {
                    return reply(6, tag, &2u32.to_le_bytes());
                }
                qids.extend_from_slice(&[0u8; 13]);
            }
            dev.fids.insert(
                newfid,
                Fid {
                    path,
                    opened: false,
                },
            );
            let mut out = (nwname as u16).to_le_bytes().to_vec();
            out.extend_from_slice(&qids);
            reply(id, tag, &out)
        }
        12 | 112 => {
            let fid = u32_at(req, &mut p);
            dev.fids.entry(fid).or_default().opened = true;
            let mut out = vec![0u8; 13];
            out.extend_from_slice(&8192u32.to_le_bytes());
            reply(id, tag, &out)
        }
        8 => {
            let mut out = Vec::new();
            out.extend_from_slice(&0x01021997u32.to_le_bytes());
            out.extend_from_slice(&8192u32.to_le_bytes());
            out.extend_from_slice(&1_000_000u64.to_le_bytes());
            out.extend_from_slice(&900_000u64.to_le_bytes());
            out.extend_from_slice(&900_000u64.to_le_bytes());
            out.extend_from_slice(&1_000_000u64.to_le_bytes());
            out.extend_from_slice(&900_000u64.to_le_bytes());
            out.extend_from_slice(&0u64.to_le_bytes());
            out.extend_from_slice(&256u16.to_le_bytes());
            reply(id, tag, &out)
        }
        116 => {
            let fid = u32_at(req, &mut p);
            let off = u64::from_le_bytes(req[p..p + 8].try_into().unwrap());
            p += 8;
            let count = u32_at(req, &mut p) as usize;
            let path = dev.fids.get(&fid).map(|f| f.path.clone());
            let mut data = Vec::new();
            if let Some(path) = path {
                if let Ok(mut f) = std::fs::File::open(path) {
                    use std::io::{Read, Seek};
                    let _ = f.seek(std::io::SeekFrom::Start(off));
                    let _ = f.take(count as u64).read_to_end(&mut data);
                }
            }
            let mut out = (data.len() as u32).to_le_bytes().to_vec();
            out.extend_from_slice(&data);
            reply(id, tag, &out)
        }
        _ => reply(6, tag, &2u32.to_le_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(id: u8, tag: u16, payload: &[u8]) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(&((payload.len() + 7) as u32).to_le_bytes());
        r.push(id);
        r.extend_from_slice(&tag.to_le_bytes());
        r.extend_from_slice(payload);
        r
    }
    fn string(value: &str) -> Vec<u8> {
        let mut r = (value.len() as u16).to_le_bytes().to_vec();
        r.extend_from_slice(value.as_bytes());
        r
    }

    #[test]
    fn ninep_host_directory_round_trip() {
        let root = std::env::temp_dir().join(format!("x86-native-9p-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("hello.txt"), b"hello").unwrap();
        let mut dev = Virtio9p {
            root: Some(root.clone()),
            ..Virtio9p::default()
        };

        let mut version = 8192u32.to_le_bytes().to_vec();
        version.extend_from_slice(&string("9P2000.L"));
        assert_eq!(handle_9p(&mut dev, &request(100, 1, &version))[4], 101);

        let mut attach = 0u32.to_le_bytes().to_vec();
        attach.extend_from_slice(&u32::MAX.to_le_bytes());
        attach.extend_from_slice(&string("root"));
        attach.extend_from_slice(&string(""));
        assert_eq!(handle_9p(&mut dev, &request(104, 2, &attach))[4], 105);

        let mut walk = 0u32.to_le_bytes().to_vec();
        walk.extend_from_slice(&1u32.to_le_bytes());
        walk.extend_from_slice(&1u16.to_le_bytes());
        walk.extend_from_slice(&string("hello.txt"));
        assert_eq!(handle_9p(&mut dev, &request(110, 3, &walk))[4], 111);

        let mut open = 1u32.to_le_bytes().to_vec();
        open.extend_from_slice(&0u32.to_le_bytes());
        assert_eq!(handle_9p(&mut dev, &request(12, 4, &open))[4], 13);

        let mut read = 1u32.to_le_bytes().to_vec();
        read.extend_from_slice(&0u64.to_le_bytes());
        read.extend_from_slice(&5u32.to_le_bytes());
        let response = handle_9p(&mut dev, &request(116, 5, &read));
        assert_eq!(response[4], 117);
        assert_eq!(&response[7 + 4..7 + 9], b"hello");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod pci_tests {
    use super::*;

    #[test]
    fn pci_config_exposes_virtio_9p_identity() {
        assert!(io_write32(PCI_CONFIG_ADDRESS, 0x8000_0000u32 as i32).to_owned());
        let id = io_read32(PCI_CONFIG_DATA).expect("PCI data port");
        assert_eq!(id as u32, 0x1009_1AF4);

        assert!(io_write32(PCI_CONFIG_ADDRESS, 0x8000_0010u32 as i32).to_owned());
        let bar = io_read32(PCI_CONFIG_DATA).expect("PCI BAR");
        assert_eq!(bar as u32, 0x0000_A001);
    }
}
