# Default mirrors PYTHON_BASE_IMAGE in versions.env (the pin authority);
# build-images.sh passes it explicitly.
ARG PYTHON_BASE_IMAGE=python:3.13.14-slim@sha256:eb43ff125d8d58d7449dcba7d336c23bcac412f526d861db493b9994d8010280
FROM ${PYTHON_BASE_IMAGE}

ARG CIRQ_VERSION
ARG QSIMCIRQ_VERSION

# qsimcirq may need a C++ toolchain if no arm64 wheel exists; install then strip.
RUN apt-get update && apt-get install -y --no-install-recommends cmake g++ make \
    && pip install --no-cache-dir \
       "cirq-core==${CIRQ_VERSION}" \
       "qsimcirq==${QSIMCIRQ_VERSION}" \
       "ply" \
    && apt-get purge -y cmake g++ make && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY run_circuit.py /app/run_circuit.py

ENTRYPOINT ["python", "/app/run_circuit.py"]
