package vvp_company.dbms_dop_test;

import lombok.extern.slf4j.Slf4j;
import org.java_websocket.client.WebSocketClient;
import org.java_websocket.handshake.ServerHandshake;

import java.net.URI;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

@Slf4j
public class DbWebSocketClient {
    private final WebSocketClient client;
    private final BlockingQueue<String> responseQueue = new LinkedBlockingQueue<>();
    private final AtomicBoolean connected = new AtomicBoolean(false);

    public DbWebSocketClient(String uri) {
        try {
            client = new WebSocketClient(URI.create(uri)) {
                @Override
                public void onOpen(ServerHandshake serverHandshake) {
                    log.debug("WebSocket открыт");
                    connected.set(true);
                }

                @Override
                public void onMessage(String message) {
                    log.debug("Получено: {}", message);
                    responseQueue.offer(message);
                }

                @Override
                public void onClose(int code, String reason, boolean remote) {
                    log.debug("WebSocket закрыт: {} {}", code, reason);
                    connected.set(false);
                }

                @Override
                public void onError(Exception ex) {
                    log.error("WebSocket ошибка", ex);
                }
            };
            client.connect();
            var start = System.currentTimeMillis();
            while (!client.isOpen() && System.currentTimeMillis() - start < 5000) {
                Thread.sleep(50);
            }
            if (!client.isOpen()) throw new RuntimeException("Не удалось подключиться");
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }

    public void send(String command) {
        if (!client.isOpen()) throw new IllegalStateException("WebSocket не открыт");
        client.send(command);
    }

    public String sendAndWaitResponse(String command, long timeoutMs) throws InterruptedException {
        send(command);
        var response = responseQueue.poll(timeoutMs, TimeUnit.MILLISECONDS);
        if (response == null) throw new RuntimeException("Нет ответа на команду: " + command);
        return response;
    }

    public void close() {
        if (client != null) client.close();
    }
}
