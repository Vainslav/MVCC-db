package vvp_company.dbms_dop_test;

import org.apache.commons.lang3.RandomStringUtils;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public abstract class IsolationTestBase {

    protected static final String WS_URI = "ws://localhost:3000/ws";
    protected List<String> usedKeys;

    @BeforeEach
    void setUpBase() {
        usedKeys = Collections.synchronizedList(new ArrayList<>());
    }

    @AfterEach
    void tearDownBase() throws Exception {
        if (usedKeys.isEmpty()) return;
        try (Session cleanup = new Session(WS_URI)) {
            cleanup.begin("serializable");
            for (var key : usedKeys) {
                try { cleanup.delete(key); } catch (Exception ignored) {}
            }
            cleanup.commit();
        } catch (Exception e) {
            System.err.println("Cleanup failed: " + e.getMessage());
        }
    }

    protected String randomKey() {
        var key = RandomStringUtils.randomAlphanumeric(10);
        usedKeys.add(key);
        return key;
    }

    protected String randomValue() {
        return RandomStringUtils.randomAlphanumeric(8);
    }
}