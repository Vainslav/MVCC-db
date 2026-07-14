```
Page:
---------------------------------------------------
| Slot[0]: {offset=XXX, key_len=xxx, val_len=xxx}  |
| Slot[1]: {offset=YYY, ...}                       |
| Slot[2]: {offset=ZZZ, ...}                       |
---------------------------------------------------
|              Free Space                          |
---------------------------------------------------
| Tuple[2] (...)                                   |  ← offset ZZZ
| Tuple[1] (...)                                   |  ← offset YYY
| Tuple[0] (xmin, xmax, key, value)                |  ← offset XXX
---------------------------------------------------
```

Page write:

```rust
static PAGE_SIZE: usize = 4096;

struct Page{
    slot_and_tuple_pointer: AtomicUsize; // usize Upper 16 slot, bottom 16 tuple
    data: SyncUnsafeCell<[u8; PAGE_SIZE]>
}

impl Page {
    pub fn write(&self, key: String, value: String) {
        slot_and_tuple_pointer ... // magic to move dependent on key and value

        while !slot_and_tuple_pointer.cas().is_ok() {};
        
        data.write(key.to_utf8());
        data.write(key.to_utf8());
    }
}

```

^
|
Bad. Problem is: Flushing Wal file is marginaly slower than holding the lock on write. So any lock-free optimizations won't provide any benefits.

But i'm gonna do it anyway :)