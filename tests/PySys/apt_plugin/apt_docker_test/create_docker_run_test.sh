sudo docker build -t "apt:Apt" .
deb_packages=$(find $HOME -type d -name debian-package_unpack)
echo ${deb_packages}
pysys_home=$(find $HOME -type d -name thin-edge.io)
echo ${pysys_home}
sudo docker run -v $pysys_home/tests/:/tests -v $deb_packages:/debian apt:Apt
echo "ran test successfully"
sudo docker  rmi --force  $(sudo docker images --filter=reference="apt:Apt" -q)