package vvp_company.dbms_dop_test;

import org.junit.jupiter.api.Test;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import static org.junit.jupiter.api.Assertions.assertEquals;

class ReadUncommittedTests extends IsolationTestBase {

    @Test
    void testDirtyReadAllowed() throws Exception {
        var key = randomKey();
        var dirtyValue = randomValue();
        var latch1 = new CountDownLatch(1);
        var latch2 = new CountDownLatch(1);
        var tx2Result = new AtomicReference<String>();

        var t1 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                s.begin("read_uncommitted");
                s.put(key, dirtyValue);
                latch1.countDown();
                var ignored = latch2.await(5, TimeUnit.SECONDS);
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        var t2 = new Thread(() -> {
            try (var s = new Session(WS_URI)) {
                var ignored = latch1.await(5, TimeUnit.SECONDS);
                s.begin("read_uncommitted");
                tx2Result.set(s.get(key));
                latch2.countDown();
                s.abort();
            } catch (Exception e) { throw new RuntimeException(e); }
        });

        t1.start(); t2.start();
        t1.join(); t2.join();
        assertEquals(dirtyValue, tx2Result.get(), "ReadUncommitted должен видеть dirty read");
    }
}