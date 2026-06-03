package vvp_company.dbms_dop_test;

import lombok.Getter;
import lombok.extern.slf4j.Slf4j;

import java.util.ArrayList;
import java.util.List;

@Slf4j
@Getter
public class Session implements AutoCloseable {
    private final DbWebSocketClient ws;
    private final List<String> history = new ArrayList<>();

    public Session(String wsUri) {
        this.ws = new DbWebSocketClient(wsUri);
    }

    public void begin(String isolation) throws Exception {
        var cmd = "begin " + isolation;
        var resp = ws.sendAndWaitResponse(cmd, 2000);
        if (!resp.equals("OK")) throw new RuntimeException("Begin failed: " + resp);
        history.add("BEGIN " + isolation);
    }

    public void put(String key, String value) throws Exception {
        var cmd = "put " + key + " " + value;
        var resp = ws.sendAndWaitResponse(cmd, 2000);
        if (!resp.equals("OK")) throw new RuntimeException("Put failed: " + resp);
        history.add("PUT " + key + "=" + value);
    }

    public String get(String key) throws Exception {
        var cmd = "get " + key;
        var resp = ws.sendAndWaitResponse(cmd, 2000);
        history.add("GET " + key + " -> " + resp);
        if (resp.startsWith("VALUE: ")) {
            return resp.substring(7);
        } else if (resp.equals("NOT_FOUND") || resp.equals("NONE_VISIBLE")) {
            return null;
        } else {
            throw new RuntimeException("Get error: " + resp);
        }
    }

    public void delete(String key) throws Exception {
        var cmd = "delete " + key;
        var resp = ws.sendAndWaitResponse(cmd, 2000);
        if (!resp.equals("OK")) throw new RuntimeException("Delete failed: " + resp);
        history.add("DELETE " + key);
    }

    public void commit() throws Exception {
        var resp = ws.sendAndWaitResponse("commit", 2000);
        if (!resp.equals("OK")) throw new RuntimeException("Commit failed: " + resp);
        history.add("COMMIT");
    }

    public void abort() throws Exception {
        var resp = ws.sendAndWaitResponse("abort", 2000);
        if (!resp.equals("OK")) throw new RuntimeException("Abort failed: " + resp);
        history.add("ABORT");
    }

    public void close() {
        ws.close();
    }
}
