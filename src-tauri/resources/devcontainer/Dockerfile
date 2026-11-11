FROM osrf/ros:jazzy-desktop

RUN apt-get update && apt-get install -y \
    python3-pip \
    python3-venv \
    python3-colcon-common-extensions \
    protobuf-compiler \
    git \
    nano \
    ros-jazzy-ros-gz \
    && rm -rf /var/lib/apt/lists/*

# Keep gRPC tooling separate from the system Python used by ROS and colcon.
RUN python3 -m venv --system-site-packages /opt/robotics-studio/venv \
    && /opt/robotics-studio/venv/bin/pip install --no-cache-dir \
    grpcio==1.85.1 \
    grpcio-tools==1.85.1 \
    setuptools==84.2.1

RUN echo "source /opt/ros/jazzy/setup.bash" >> /root/.bashrc

WORKDIR /workspace

# Copy protobuf definitions
COPY proto ./proto