dpkg -i /debian/*.deb; apt-get install -y -f
cd /tests/PySys; pysys.py run apt_remove
exit
