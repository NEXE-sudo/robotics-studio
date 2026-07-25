FROM osrf/ros:jazzy-desktop

RUN apt-get update && apt-get install -y \
    python3-pip \
    python3-colcon-common-extensions \
    protobuf-compiler \
    git \
    nano \
    ros-jazzy-ros-gz \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --break-system-packages grpcio grpcio-tools

RUN echo "source /opt/ros/jazzy/setup.bash" >> /root/.bashrc

WORKDIR /workspace

# Copy protobuf definitions
COPY proto ./proto