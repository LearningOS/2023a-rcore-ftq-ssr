//! Implementation of [`PageTableEntry`] and [`PageTable`].
use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

// use super::memory_set::MapPermission;
// use crate::task::{remove_current_maparea,create_current_maparea,};

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    /// bits of page table entry
    pub bits: usize,
}

impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    /// Create an empty page table entry
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    /// Get the physical page number from the page table entry
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    /// Get the flags from the page table entry
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    /// The page pointered by page table entry is valid?
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is readable?
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is writable?
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    /// The page pointered by page table entry is executable?
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create a new page table
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find PageTableEntry by VirtPageNum, create a frame for a 4KB page table if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find PageTableEntry by VirtPageNum
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// set the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    /// remove the map between virtual page number and physical page number
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    /// get the page table entry from the virtual page number
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// get the physical address from the virtual address
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = pte.ppn().into();
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    /// get the token from the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

// /// alloc the mmap between virtual addr and physical addr
// pub fn create_mmap(token: usize,start:usize, len: usize, _port: usize) -> isize{
//     let page_table = PageTable::from_token(token);
//     let start_va = VirtAddr::from(start);
//     // let vpn = start_va.floor();
//     let end = start + len;
//     let end_va = VirtAddr::from(end);
//     let mut flags=MapPermission::U;
//     if _port&(1<<0)!=0 {flags|=MapPermission::R;}
//     if _port&(1<<1)!=0 {flags|=MapPermission::W;}
//     if _port&(1<<2)!=0 {flags|=MapPermission::X;}
//     for i in (start..end).step_by(PAGE_SIZE).into_iter(){
//         if let Some(x)=page_table.translate(VirtAddr::from(i).floor()) {
//             if x.is_valid() {return -1;}
//         }
//     }
//     create_current_maparea(start_va,end_va,flags);
//     0
// }
// /// remove the mmap between virtual addr and physical addr
// pub fn remove_mmap(token: usize,start:usize, len: usize) -> isize{
//     let mut page_table = PageTable::from_token(token);
//     let start_va = VirtAddr::from(start);
//     // let vpn = start_va.floor();
//     let end = start + len;
//     let end_va = VirtAddr::from(end);
//     for i in (start..end).step_by(PAGE_SIZE).into_iter(){
//         if let None=page_table.translate(VirtAddr::from(i).floor()) {return -1;}
//     }
//     if remove_current_maparea(&mut page_table, start_va,end_va)==false {return -1;}
//     0
// }
// /// Translate&Copy mut to a mutable usize through page table
// pub fn change_mut_usize(token: usize, ptr: usize,data:usize){
//     let page_table = PageTable::from_token(token);
//     let start = ptr as usize;
//     let start_va = VirtAddr::from(start);
//     let vpn = start_va.floor(); 
//     let ppn = page_table.translate(vpn).unwrap().ppn();
//     let usiz_array: & 'static mut [usize; crate::config::PAGE_SIZE/core::mem::size_of::<usize>()] = ppn.get_mut();
//     usiz_array[start_va.page_offset()/core::mem::size_of::<usize>()] = data;
// }
// /// Translate&Copy mut to a mutable u32 through page table
// pub fn change_mut_u32(token: usize, ptr: usize,data:u32){
//     let page_table = PageTable::from_token(token);
//     let start = ptr as usize;
//     let start_va = VirtAddr::from(start);
//     let vpn = start_va.floor();
//     let ppn = page_table.translate(vpn).unwrap().ppn();
//     let usiz_array: & 'static mut [u32; crate::config::PAGE_SIZE/core::mem::size_of::<u32>()] = ppn.get_mut();
//     usiz_array[start_va.page_offset()/core::mem::size_of::<u32>()] = data;
// }/// Translate&Copy mut to a mutable u8 through page table
// pub fn change_mut_u8(token: usize, ptr: usize,data:u8){
//     let page_table = PageTable::from_token(token);
//     let start = ptr as usize;
//     let start_va = VirtAddr::from(start);
//     let vpn = start_va.floor();
//     let ppn = page_table.translate(vpn).unwrap().ppn();
//     let usiz_array: & 'static mut [u8; crate::config::PAGE_SIZE/core::mem::size_of::<u8>()] = ppn.get_mut();
//     usiz_array[start_va.page_offset()/core::mem::size_of::<u8>()] = data;
// }
/// Translate&Copy a ptr[u8] array with LENGTH len to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// Translate&Copy a ptr[u8] array end with `\0` to a `String` Vec through page table
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}
/// Translate a ptr[u8] array through page table and return a mutable reference of T
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    //trace!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    //trace!("translated_refmut: before translate_va");
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}
/// Translate&Copy mut to a mutable usize through page table
pub fn change_mut_usize(token: usize, ptr: usize,data:usize){
    let page_table = PageTable::from_token(token);
    let start = ptr as usize;
    let start_va = VirtAddr::from(start);
    let vpn = start_va.floor(); 
    let ppn = page_table.translate(vpn).unwrap().ppn();
    let usiz_array: & 'static mut [usize; crate::config::PAGE_SIZE/core::mem::size_of::<usize>()] = ppn.get_mut();
    usiz_array[start_va.page_offset()/core::mem::size_of::<usize>()] = data;
}