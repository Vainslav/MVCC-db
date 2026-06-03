package vvp_company.dbms_dop_test;

import org.apache.commons.lang3.RandomStringUtils;
import org.junit.jupiter.api.Test;
import java.util.concurrent.*;
import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;

class StressTests extends IsolationTestBase {

    @Test
    void randomStressTest() {
        var numThreads = 5;
        var txPerThread = 20;
        var executor = Executors.newFixedThreadPool(numThreads);
        var futures = new Future[numThreads];

        for (var i = 0; i < numThreads; i++) {
            futures[i] = executor.submit(() -> {
                try (var s = new Session(WS_URI)) {
                    for (var t = 0; t < txPerThread; t++) {
                        var isolations = new String[]{"read_uncommitted", "read_committed", "repeatable_read", "serializable"};
                        var isolation = isolations[ThreadLocalRandom.current().nextInt(4)];
                        s.begin(isolation);
                        var ops = ThreadLocalRandom.current().nextInt(1, 4);
                        for (var op = 0; op < ops; op++) {
                            var localKey = RandomStringUtils.randomAlphanumeric(10);
                            var action = ThreadLocalRandom.current().nextInt(3);
                            try {
                                if (action == 0) s.get(localKey);
                                else if (action == 1) s.put(localKey, RandomStringUtils.randomAlphanumeric(8));
                                else s.delete(localKey);
                            } catch (RuntimeException e) {
                                var msg = e.getMessage();
                                if (!msg.contains("NOT_FOUND") && !msg.contains("NONE_VISIBLE")) {
                                    throw e;
                                }
                            }
                        }
                        if (ThreadLocalRandom.current().nextDouble() < 0.7) {
                            try {
                                s.commit();
                            } catch (RuntimeException e) {
                                if (!e.getMessage().contains("SerializationError")) throw e;
                            }
                        } else {
                            s.abort();
                        }
                    }
                    return s.getHistory();
                } catch (Exception e) { throw new RuntimeException(e); }
            });
        }
        for (var f : futures) {
            assertDoesNotThrow(() -> f.get(), "Стресс-тест упал с ошибкой");
        }
        executor.shutdown();
    }
}