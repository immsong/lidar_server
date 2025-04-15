# 안드로이드 기기에서 Rust 코드 테스트 가이드

## 1. 기본 설정

```bash
# 안드로이드 타겟 추가
rustup target add aarch64-linux-android

# NDK 경로 설정
export ANDROID_NDK_HOME=/home/yssong/Android/Sdk/ndk/29.0.13113456
export PATH=$PATH:$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin
```

## 2. 안드로이드 기기 연결

```bash
# USB 디버깅 활성화 확인
adb devices
# 기기가 목록에 표시되어야 함
```

## 3. Rust 프로젝트 빌드

```bash
# 안드로이드 타겟으로 빌드
cargo build --target aarch64-linux-android
```

## 4. 바이너리 전송 및 실행

```bash
# 바이너리를 기기에 전송
adb push target/aarch64-linux-android/debug/[바이너리_이름] /data/local/tmp/

# 기기에서 실행
adb shell
cd /data/local/tmp
chmod +x [바이너리_이름]
./[바이너리_이름]
```

## 5. 로그 확인

```bash
# 별도의 터미널에서 로그 확인
adb logcat | grep Rust
```

## 6. 권한 설정

```bash
# 네트워크 권한 확인
adb shell pm list permissions | grep -E "INTERNET|MULTICAST"
```

## 7. 디버깅 팁

```bash
# 프로세스 확인
adb shell ps | grep [바이너리_이름]

# 네트워크 상태 확인
adb shell netstat

# 로그 파일 확인
adb shell logcat -d > android_log.txt
```

## 8. 문제 해결

### 권한 문제 발생 시
```bash
adb shell
su  # 루트 권한 필요
```

### 네트워크 문제 발생 시
```bash
adb shell ifconfig
adb shell netcfg
```

## 주의사항
- 실제 안드로이드 기기에서 네트워크 관련 기능을 테스트할 때는 권한 설정이 중요합니다.
- USB 디버깅이 활성화되어 있어야 합니다.
- 루트 권한이 필요한 경우가 있을 수 있습니다.
