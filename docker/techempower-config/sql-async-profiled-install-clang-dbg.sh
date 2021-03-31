mkdir /tmp/profile-data

rm -rf $IROOT/ffead-cpp-5.0-sql

if [ "$1" = "batch" ]
then
	apt remove -yqq libpq-dev
	apt autoremove -yqq
	rm -f /usr/lib/x86_64-linux-gnu/libpq.*
	apt update && apt install -y bison flex libreadline-dev
	cd /tmp
	#wget -q https://github.com/an-tao/postgres/archive/batch_mode_ubuntu.tar.gz
	#tar -xzf batch_mode_ubuntu.tar.gz
	#cd postgres-batch_mode_ubuntu
	#./configure --prefix=/usr CFLAGS='-O2 -pipe -march=native'
	#make && make install
	wget -nv https://github.com/postgres/postgres/archive/b787d4ce6d910080065025bcd5f968544997271f.zip
	unzip -q b787d4ce6d910080065025bcd5f968544997271f.zip
	cd postgres-b787d4ce6d910080065025bcd5f968544997271f
	wget -nv https://www.postgresql.org/message-id/attachment/115223/v22-0001-libpq-batch.patch
	git apply ./v22-0001-libpq-batch.patch
	./configure --prefix=/usr CFLAGS='-O3 -march=native -flto'
	cd src/interfaces/libpq
	make all install -j4
	cp ../../../src/include/postgres_ext.h ../../../src/include/pg_config_ext.h libpq-fe.h /usr/include
fi

apt update -yqq && apt install -yqq clang

cd $IROOT/ffead-cpp-src/
rm -rf CMakeCache.txt CMakeFiles
rm -rf web/te-benchmark-um web/te-benchmark-um-mgr web/te-benchmark-um-pq

sed -i 's|add_subdirectory(${PROJECT_SOURCE_DIR}/web/te-benchmark-um)||g' CMakeLists.txt
sed -i 's|add_subdirectory(${PROJECT_SOURCE_DIR}/web/te-benchmark-um-mgr)||g' CMakeLists.txt
sed -i 's|add_subdirectory(${PROJECT_SOURCE_DIR}/web/te-benchmark-um-pq)||g' CMakeLists.txt
sed -i 's|install(FILES ${PROJECT_BINARY_DIR}/web/te-benchmark-um/libte_benchmark_um${LIB_EXT} DESTINATION ${PROJECT_NAME}-bin/lib)||g' CMakeLists.txt
sed -i 's|install(FILES ${PROJECT_BINARY_DIR}/web/te-benchmark-um-mgr/libte_benchmark_um_mgr${LIB_EXT} DESTINATION ${PROJECT_NAME}-bin/lib)||g' CMakeLists.txt
sed -i 's|install(FILES ${PROJECT_BINARY_DIR}/web/te-benchmark-um-pq/libte_benchmark_um_pq${LIB_EXT} DESTINATION ${PROJECT_NAME}-bin/lib)||g' CMakeLists.txt

sed -i 's|tfb-database|localhost|g' $IROOT/ffead-cpp-src/web/te-benchmark-um-pq-async/config/sdorm.xml

rm -rf build
mkdir build
cd build
CC=/usr/bin/clang CXX=/usr/bin/clang++ cmake -DSRV_EMB=on -DMOD_REDIS=on -DDEBUG=on ..
make install && mv $IROOT/ffead-cpp-src/ffead-cpp-5.0-bin $IROOT/ffead-cpp-sql-raw

#Start postgresql
service postgresql stop
#For profiling/benchmarking

cd $IROOT/
sed -i 's|cmake |CC=/usr/bin/clang CXX=/usr/bin/clang++ cmake |g' $IROOT/ffead-cpp-sql-raw/resources/rundyn-automake.sh
#sed -i 's|-fprofile-instr-generate=/tmp/cprof.prof|-fprofile-instr-generate=/tmp/cprofdi.prof|g' $IROOT/ffead-cpp-sql-raw/rtdcf/CMakeLists.txt.template
apt update && apt install vim gdb
./install_ffead-cpp-sql-raw-profiled.sh async
