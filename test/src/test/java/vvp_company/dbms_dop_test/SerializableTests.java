package vvp_company.dbms_dop_test;

import org.junit.jupiter.api.Test;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import static org.junit.jupiter.api.Assertions.*;

class SerializableTests extends IsolationTestBase {

    @Test
    void testLostUpdateNotAllowed() throws Exception {
        var key = randomKey();
        try (var prep = new Session(WS_URI)) {
            prep.begin("serializable");
            prep.put(key, "10");
            prep.commit();
        }

        var bothRead = new CountDownLatch(2);
        var ex1 = new AtomicReference<Exception>();
        var ex2 = new AtomicReference<Exception>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("serializable");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 1));
                s.commit();
            } catch (Exception e) { ex1.set(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("serializable");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 1));
                s.commit();
            } catch (Exception e) { ex2.set(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();

        var t1Failed = ex1.get() != null && ex1.get().getMessage().contains("SerializationError");
        var t2Failed = ex2.get() != null && ex2.get().getMessage().contains("SerializationError");
        assertTrue(t1Failed || t2Failed, "Serializable не должен допускать lost update");

        try (var check = new Session(WS_URI)) {
            check.begin("serializable");
            var finalVal = check.get(key);
            check.abort();
            assertEquals("11", finalVal);
        }
    }

    @Test
    void testWriteSkewNotAllowed() throws Exception {
        var keyX = randomKey();
        var keyY = randomKey();
        try (var prep = new Session(WS_URI)) {
            prep.begin("serializable");
            prep.put(keyX, "100");
            prep.put(keyY, "100");
            prep.commit();
        }

        var bothRead = new CountDownLatch(2);
        var ex1 = new AtomicReference<Exception>();
        var ex2 = new AtomicReference<Exception>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("serializable");
                var x = s.get(keyX);
                var y = s.get(keyY);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                if (x.equals(y)) s.put(keyX, "150");
                s.commit();
            } catch (Exception e) { ex1.set(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("serializable");
                var x = s.get(keyX);
                var y = s.get(keyY);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                if (x.equals(y)) s.put(keyY, "150");
                s.commit();
            } catch (Exception e) { ex2.set(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();

        var t1Failed = ex1.get() != null && ex1.get().getMessage().contains("SerializationError");
        var t2Failed = ex2.get() != null && ex2.get().getMessage().contains("SerializationError");
        assertTrue(t1Failed || t2Failed, "Serializable не должен допускать write-skew");

        try (var check = new Session(WS_URI)) {
            check.begin("serializable");
            var x = check.get(keyX);
            var y = check.get(keyY);
            check.abort();
            assertFalse("150".equals(x) && "150".equals(y), "Write-skew произошёл: оба 150");
        }
    }
}
