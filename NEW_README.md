# Architecture

I am going to use Hashed files With LSM inspired data files.

```
Index file:
 ---------------------------
|[Header]                   |
|Bucket_count: 32 bits      |
|                           |
|[Bucket page 0]            |
|[Bucket page 1]            |
|[Bucket page 2]            |
|[Bucket page 3]            |
|...                        |
 ---------------------------

Bucket page:
 -----------------------------------------------------------------------------------------------------------------------------------------
|next_overflow_page: 32 bits (0 if no overflow)                                                                                           |
|entry_count: 16 bits                                                                                                                     |
|                                                                                                                                         |
|[Entry 0] {hash: 64 bits, key_len: 16 bits, key_bytes: [...], tid: (seg: 32 bits, offset: 64 bits)}                                      |
|[Entry 1]                                                                                                                                |
|[Entry 2]                                                                                                                                |
|[Entry 3]                                                                                                                                |
|...                                                                                                                                      |
 -----------------------------------------------------------------------------------------------------------------------------------------
```

```
Data file:
 ---------------------------
|[Header]                   |
|Page count: 32 bits        |
|                           |
|[Data page 0]              |
|[Data page 1]              |
|[Data page 2]              |
|[Data page 3]              |
|...                        |
 ---------------------------

Data page:
 -----------------------------------------------------------------------------------------------------------------------------------------
|entry_count: 16 bits                                                                                                                     |
|                                                                                                                                         |
|[Record 0] {xmin: 32 bits, xmax: 32 bits, data_len: 16 bits, data: [...] prev_tid: (seg: 32 bits, offset: 64 bits)}                      |
|[Record 1]                                                                                                                               |
|[Record 2]                                                                                                                               |
|[Record 3]                                                                                                                               |
|...                                                                                                                                      |
 -----------------------------------------------------------------------------------------------------------------------------------------
```
