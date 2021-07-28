dpkg -i /debian/*.deb; apt-get install -y -f
cd /tests/PySys; pysys.py run apt_remove apt_sequence_install_with_version apt_sequence_remove_with_version
exit
