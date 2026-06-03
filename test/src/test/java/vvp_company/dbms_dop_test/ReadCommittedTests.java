package vvp_company.dbms_dop_test;

import org.junit.jupiter.api.Test;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import static org.junit.jupiter.api.Assertions.*;

class ReadCommittedTests extends IsolationTestBase {

    @Test
    void testDirtyReadNotAllowed() throws Exception {
        var key = randomKey();
        var dirtyValue = randomValue();
        var latch1 = new CountDownLatch(1);
        var latch2 = new CountDownLatch(1);
        var tx2Result = new AtomicReference<String>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("read_committed");
                s.put(key, dirtyValue);
                latch1.countDown();
                var ignored = latch2.await(5, TimeUnit.SECONDS);
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                var ignored = latch1.await(5, TimeUnit.SECONDS);
                s.begin("read_committed");
                tx2Result.set(s.get(key));
                latch2.countDown();
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();
        assertNull(tx2Result.get(), "ReadCommitted не должен видеть dirty read");
    }

    @Test
    void testNonRepeatableReadAllowed() throws Exception {
        var key = randomKey();
        var initial = randomValue();
        try (var prep = new Session(WS_URI)) {
            prep.begin("read_committed");
            prep.put(key, initial);
            prep.commit();
        }

        var firstRead = new AtomicReference<String>();
        var secondRead = new AtomicReference<String>();
        var latch = new CountDownLatch(1);

        var tx1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("read_committed");
                firstRead.set(s.get(key));
                var ignored = latch.await(5, TimeUnit.SECONDS);
                secondRead.set(s.get(key));
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        var tx2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                Thread.sleep(500);
                s.begin("read_committed");
                s.put(key, randomValue());
                s.commit();
                latch.countDown();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        tx1.start(); tx2.start();
        tx1.join(); tx2.join();
        assertEquals(initial, firstRead.get());
        assertNotEquals(firstRead.get(), secondRead.get(), "ReadCommitted должен допускать non-repeatable read");
    }

    @Test
    void testLostUpdateAllowed() throws Exception {
        var key = randomKey();
        try (var prep = new Session(WS_URI)) {
            prep.begin("read_committed");
            prep.put(key, "10");
            prep.commit();
        }

        var bothRead = new CountDownLatch(2);
        var ex1 = new AtomicReference<Exception>();
        var ex2 = new AtomicReference<Exception>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("read_committed");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 1));
                s.commit();
            } catch (Exception e) { ex1.set(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("read_committed");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 10));
                s.commit();
            } catch (Exception e) { ex2.set(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();
        assertNull(ex1.get());
        assertNull(ex2.get());

        try (var check = new Session(WS_URI)) {
            check.begin("read_committed");
            var finalVal = check.get(key);
            check.commit();
            assertTrue(finalVal.equals("11") || finalVal.equals("20"), "Lost update должен быть возможен, final=" + finalVal);
        }
    }
}