# Default mirrors PYTHON_BASE_IMAGE in versions.env (the pin authority);
# build-images.sh passes it explicitly.
ARG PYTHON_BASE_IMAGE=python:3.13.14-slim@sha256:eb43ff125d8d58d7449dcba7d336c23bcac412f526d861db493b9994d8010280
FROM ${PYTHON_BASE_IMAGE}

ARG QISKIT_VERSION
ARG QISKIT_AER_VERSION

RUN pip install --no-cache-dir \
    "qiskit==${QISKIT_VERSION}" \
    "qiskit-aer==${QISKIT_AER_VERSION}"

WORKDIR /app
COPY run_circuit.py /app/run_circuit.py

ENTRYPOINT ["python", "/app/run_circuit.py"]
