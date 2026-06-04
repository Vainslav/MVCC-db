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
The database is tested with a comprehensive suite of Java-based tests that verify isolation level guarantees and serializability.
- Isolation Test Base (IsolationTestBase)
- Read Uncommitted Tests
    - testDirtyReadAllowed – confirms that a transaction can read uncommitted changes from another transaction (dirty read is permitted).
- Read Committed Tests (ReadCommittedTests)
    - testDirtyReadNotAllowed – ensures that dirty reads are prevented.
    - testNonRepeatableReadAllowed – shows that non‑repeatable reads (different values for the same key within a transaction) can occur.
    - testLostUpdateAllowed – verifies that lost updates (two concurrent transactions overwriting each other’s changes) are possible under Read Committed.
- Repeatable Read Tests (RepeatableReadTests)
    - testDirtyReadNotAllowed – dirty reads are blocked.
    - testNonRepeatableReadNotAllowed – repeatable reads are guaranteed (same key returns same value throughout the transaction).
    - testLostUpdateAllowed – lost updates are still possible (no automatic conflict detection).
    - testWriteSkewAllowed – write skew (two transactions reading overlapping data and updating disjoint keys) is permitted.
- Serializable Tests (SerializableTests)
    - testLostUpdateNotAllowed – ensures lost updates are prevented; at least one transaction fails with SerializationError.
    - testWriteSkewNotAllowed – write skew is blocked; only one transaction may succeed, or both fail if they conflict.
- Serializable Graph Test (SerializableGraphTest)
    - testNoCyclesInSerializableSchedule – runs 1000 random serializable transactions and builds a dependency graph based on read/write sets. Verifies that the graph contains no cycles, which is a necessary condition for serializability (conflict serializability). If a cycle is detected, the schedule is not serializable.
- Stress Tests (StressTests)
    - randomStressTest – launches multiple threads, each executing random transactions with random isolation levels, operations (GET/PUT/DELETE), and random commit/abort decisions. The test ensures no unexpected exceptions (like panics or protocol errors) occur under concurrent load.

# Future Improvements
- Implement vacuum