package vvp_company.dbms_dop_test;

import org.junit.jupiter.api.Test;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import static org.junit.jupiter.api.Assertions.*;

class RepeatableReadTests extends IsolationTestBase {

    @Test
    void testDirtyReadNotAllowed() throws Exception {
        var key = randomKey();
        var dirtyValue = randomValue();
        var latch1 = new CountDownLatch(1);
        var latch2 = new CountDownLatch(1);
        var tx2Result = new AtomicReference<String>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("repeatable_read");
                s.put(key, dirtyValue);
                latch1.countDown();
                var ignored = latch2.await(5, TimeUnit.SECONDS);
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                var ignored = latch1.await(5, TimeUnit.SECONDS);
                s.begin("repeatable_read");
                tx2Result.set(s.get(key));
                latch2.countDown();
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();
        assertNull(tx2Result.get(), "RepeatableRead не должен видеть dirty read");
    }

    @Test
    void testNonRepeatableReadNotAllowed() throws Exception {
        var key = randomKey();
        var initial = randomValue();
        try (var prep = new Session(WS_URI)) {
            prep.begin("repeatable_read");
            prep.put(key, initial);
            prep.commit();
        }

        var firstRead = new AtomicReference<String>();
        var secondRead = new AtomicReference<String>();
        var latch = new CountDownLatch(1);

        var tx1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("repeatable_read");
                firstRead.set(s.get(key));
                var ignored = latch.await(5, TimeUnit.SECONDS);
                secondRead.set(s.get(key));
                s.commit();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        var tx2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                Thread.sleep(500);
                s.begin("repeatable_read");
                s.put(key, randomValue());
                s.commit();
                latch.countDown();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        tx1.start(); tx2.start();
        tx1.join(); tx2.join();
        assertEquals(initial, firstRead.get());
        assertEquals(initial, secondRead.get(), "RepeatableRead не должен допускать non-repeatable read");
    }

    @Test
    void testLostUpdateAllowed() throws Exception {
        var key = randomKey();
        try (var prep = new Session(WS_URI)) {
            prep.begin("repeatable_read");
            prep.put(key, "10");
            prep.commit();
        }

        var bothRead = new CountDownLatch(2);
        var ex1 = new AtomicReference<Exception>();
        var ex2 = new AtomicReference<Exception>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("repeatable_read");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 1));
                s.commit();
            } catch (Exception e) { ex1.set(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("repeatable_read");
                var val = s.get(key);
                bothRead.countDown();
                var ignored = bothRead.await(5, TimeUnit.SECONDS);
                s.put(key, String.valueOf(Integer.parseInt(val) + 1));
                s.commit();
            } catch (Exception e) { ex2.set(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();
        assertNull(ex1.get());
        assertNull(ex2.get());

        try (var check = new Session(WS_URI)) {
            check.begin("repeatable_read");
            var finalVal = check.get(key);
            check.commit();
            assertTrue(finalVal.equals("11") || finalVal.equals("12"), "Lost update должен быть возможен в RepeatableRead, final=" + finalVal);
        }
    }

    @Test
    void testWriteSkewAllowed() throws Exception {
        var keyX = randomKey();
        var keyY = randomKey();
        try (var prep = new Session(WS_URI)) {
            prep.begin("repeatable_read");
            prep.put(keyX, "100");
            prep.put(keyY, "100");
            prep.commit();
        }

        var bothRead = new CountDownLatch(2);
        var ex1 = new AtomicReference<Exception>();
        var ex2 = new AtomicReference<Exception>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("repeatable_read");
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
                s.begin("repeatable_read");
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

        assertNull(ex1.get());
        assertNull(ex2.get());

        try (var check = new Session(WS_URI)) {
            check.begin("repeatable_read");
            var x = check.get(keyX);
            var y = check.get(keyY);
            check.commit();
            // Write skew допустим – могут быть оба 150
            boolean both150 = "150".equals(x) && "150".equals(y);
            boolean one150 = ("150".equals(x) && "100".equals(y)) || ("100".equals(x) && "150".equals(y));
            assertTrue(both150 || one150, "Write skew должен быть возможен в RepeatableRead");
        }
    }
}