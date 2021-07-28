FROM ubuntu:20.04

RUN apt-get update -y

# Install packages
RUN apt-get install -y python3.8
RUN apt-get install -y python3-pip
RUN pip3 install pysys
RUN apt-get install sudo
RUN apt-get --assume-yes install mosquitto

CMD sh /tests/PySys/apt_plugin/apt_docker_test/install.sh