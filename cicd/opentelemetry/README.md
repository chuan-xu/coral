# 启动OpenTelemetry

1. docker 启动

```bash
docker run -v collector.yaml:/etc/otelcol-contrib/config.yaml -p 4317:4317 -p 4318:4318 otel/opentelemetry-collector-contrib:latest otel
```

# 启动Jaeger

```bash
./jaeger-all-in-one --collector.otlp.grpc.host-port 0.0.0.0:4317  --collector.otlp.http.host-port 0.0.0.0:4318
```