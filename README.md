# Transactional In‑Memory Database with MVCC
This project implements an in‑memory database core with full ACID transactions, multiversion concurrency control (MVCC), and ANSI SQL isolation levels. The database is accessed programmatically via the Connection API and the execute_command function.

# Features
- Transactions – BEGIN, COMMIT, ABORT.
- Isolation levels (ANSI SQL):
    - READ UNCOMMITTED
    - READ COMMITTED
    - REPEATABLE READ
    - SERIALIZABLE
- Data operations:
    - PUT <key> <value> – insert or update a value.
    - GET <key> – read a value according to visibility rules.
    - DELETE <key> – delete a key.
- Internal model:
    - MVCC – each value version stores tx_start and tx_end.
    - Conflict detection for SERIALIZABLE (intersection of read and write sets).
    - No locks – visibility is determined solely by transaction metadata.
- Concurrency – shared storage and transaction manager; each Connection has its own current transaction.

# Errors
- NoActiveTransaction – attempting PUT/GET/DELETE/COMMIT/ABORT outside a transaction.
- TransactionAlreadyActive – a second BEGIN without COMMIT/ABORT.
- SerializationError – committing a SERIALIZABLE transaction that conflicts (e.g., another transaction modified data that the current one read).
- NotFound – the key has never existed.
- NoneVisible – the key exists, but no version is visible to the current transaction (due to isolation level or deletion).

# Testing 
The db module contains unit tests for isolation levels and transaction manager behavior.

# Future Improvements
- Implement vacuum