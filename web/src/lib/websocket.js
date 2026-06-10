export class WebSocketClient {
    constructor() {
        this.ws = null;
        this.callbacks = [];
        this.reconnectInterval = 3000;
    }
    connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        this.ws = new WebSocket(wsUrl);
        this.ws.onopen = () => {
            console.log('WebSocket connected');
        };
        this.ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                if (data.type === 'event') {
                    this.callbacks.forEach(cb => cb(data.event));
                }
            }
            catch (e) {
                console.error('Failed to parse WebSocket message:', e);
            }
        };
        this.ws.onclose = () => {
            console.log('WebSocket disconnected, reconnecting...');
            setTimeout(() => this.connect(), this.reconnectInterval);
        };
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };
    }
    onEvent(callback) {
        this.callbacks.push(callback);
        return () => {
            this.callbacks = this.callbacks.filter(cb => cb !== callback);
        };
    }
    send(data) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        }
    }
    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }
}
export const wsClient = new WebSocketClient();
