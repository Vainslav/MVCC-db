package vvp_company.dbms_dop_test;

import org.junit.jupiter.api.Test;
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.atomic.AtomicInteger;
import static org.junit.jupiter.api.Assertions.*;

public class SerializableGraphTest extends IsolationTestBase {

    @Test
    void testNoCyclesInSerializableSchedule() throws Exception {
        int numTransactions = 1000;
        int keysRange = 5;
        int maxOpsPerTx = 4;
        int threads = 5;
        ExecutorService executor = Executors.newFixedThreadPool(threads);
        List<Future<TxInfo>> futures = new ArrayList<>();

        for (int i = 0; i < numTransactions; i++) {
            futures.add(executor.submit(() -> runRandomSerializableTx(keysRange, maxOpsPerTx)));
        }

        List<TxInfo> committedTxs = new ArrayList<>();
        for (Future<TxInfo> f : futures) {
            try {
                TxInfo info = f.get();
                if (info != null) {
                    committedTxs.add(info);
                }
            } catch (ExecutionException e) {
                Throwable cause = e.getCause();
                if (cause != null && cause.getMessage().contains("SerializationError")) {
                    continue;
                }
                throw e;
            }
        }
        executor.shutdown();
        assertFalse(committedTxs.isEmpty(), "Нет успешных транзакций для анализа");

        assertTrue(isAcyclic(committedTxs),
                "Обнаружен цикл в графе зависимостей → нарушение сериализуемости");
    }

    private TxInfo runRandomSerializableTx(int keysRange, int maxOps) throws Exception {
        try (Session s = new Session(WS_URI)) {
            s.begin("serializable");
            Set<String> readSet = new HashSet<>();
            Set<String> writeSet = new HashSet<>();
            int ops = ThreadLocalRandom.current().nextInt(1, maxOps + 1);
            for (int i = 0; i < ops; i++) {
                String key = "key" + ThreadLocalRandom.current().nextInt(keysRange);
                int type = ThreadLocalRandom.current().nextInt(2);
                if (type == 0) {
                    s.get(key);
                    readSet.add(key);
                } else {
                    s.put(key, randomValue());
                    writeSet.add(key);
                }
            }
            s.commit();
            return new TxInfo(readSet, writeSet);
        }
    }

    private boolean isAcyclic(List<TxInfo> txs) {
        int n = txs.size();
        Map<Integer, Set<Integer>> graph = new HashMap<>();
        for (int i = 0; i < n; i++) graph.put(i, new HashSet<>());

        for (int i = 0; i < n; i++) {
            for (int j = i + 1; j < n; j++) {
                TxInfo ti = txs.get(i);
                TxInfo tj = txs.get(j);
                if (!Collections.disjoint(ti.writeSet, tj.readSet)) graph.get(i).add(j);
                if (!Collections.disjoint(ti.readSet, tj.writeSet)) graph.get(i).add(j);
                if (!Collections.disjoint(ti.writeSet, tj.writeSet)) graph.get(i).add(j);
            }
        }

        int[] visited = new int[n];
        for (int i = 0; i < n; i++) {
            if (visited[i] == 0 && hasCycle(i, graph, visited)) {
                return false;
            }
        }
        return true;
    }

    private boolean hasCycle(int v, Map<Integer, Set<Integer>> graph, int[] visited) {
        visited[v] = 1;
        for (int u : graph.get(v)) {
            if (visited[u] == 1) return true;
            if (visited[u] == 0 && hasCycle(u, graph, visited)) return true;
        }
        visited[v] = 2;
        return false;
    }

    static class TxInfo {
        Set<String> readSet;
        Set<String> writeSet;
        TxInfo(Set<String> readSet, Set<String> writeSet) {
            this.readSet = readSet;
            this.writeSet = writeSet;
        }
    }
}