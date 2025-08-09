package_windows64:
	# Build ilass
	cargo build --release --bin ilass --target x86_64-pc-windows-gnu

	# Create package structure
	mkdir -p pkg
	mkdir -p pkg/ffmpeg

	# Copy FFmpeg files
	curl -L https://github.com/GyanD/codexffmpeg/releases/download/5.0/ffmpeg-5.0-essentials_build.zip -o target/ffmpeg.zip
	unzip target/ffmpeg.zip -d target
	cp target/ffmpeg-5.0-essentials_build/LICENSE pkg/ffmpeg/LICENSE-ffmpeg
	cp target/ffmpeg-5.0-essentials_build/README.txt pkg/ffmpeg/README.txt
	cp target/ffmpeg-5.0-essentials_build/bin/ffmpeg.exe pkg/ffmpeg/ffmpeg.exe
	cp target/ffmpeg-5.0-essentials_build/bin/ffprobe.exe pkg/ffmpeg/ffprobe.exe

	# Copy ilass files
	cp LICENSE pkg/LICENSE
	cp target/x86_64-pc-windows-gnu/release/ilass.exe pkg/ilass.exe

	# Create batch script
	echo -ne '@echo off\r\nset ILASS_FFMPEG_PATH=%~dp0ffmpeg\\ffmpeg.exe\r\nset ILASS_FFPROBE_PATH=%~dp0ffmpeg\\ffprobe.exe\r\n"%~dp0ilass.exe" %*\r\n' > target/ilass-windows64/ilass.bat

	# Create zip archive
	zip -r -j ilass-win-x86_64.zip pkg

clean_windows64:
	rm -rf target/ffmpeg-*
	rm -rf pkg
	rm -f ilass-win-x86_64.zip

package_linux64:
	cargo build --release --bin ilass --target x86_64-unknown-linux-musl
	tar -czvf ilass-linux-x86_64.tar.gz --dir target/x86_64-unknown-linux-musl/release ilass
