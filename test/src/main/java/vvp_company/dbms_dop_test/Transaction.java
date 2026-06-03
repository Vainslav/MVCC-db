package vvp_company.dbms_dop_test;

import java.util.ArrayList;
import java.util.List;

public class Transaction {
    private final List<Operation> operations = new ArrayList<>();

    public Transaction put(String key, String value) {
        operations.add(new Operation(OpType.PUT, key, value));
        return this;
    }

    public Transaction get(String key) {
        operations.add(new Operation(OpType.GET, key, null));
        return this;
    }

    public Transaction delete(String key) {
        operations.add(new Operation(OpType.DELETE, key, null));
        return this;
    }

    public List<Operation> getOperations() {
        return operations;
    }
}
