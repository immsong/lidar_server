# Lidar Server

## 프로젝트 개요
- LiDAR(Light Detection and Ranging, 거리 측정 센서) 데이터를 처리하고 제공하는 서버 애플리케이션
- 32비트/64비트 Windows 환경 지원
- MSVC 빌드 시스템 사용

## 기술 스택
- 언어: Rust
- 빌드 시스템: Cargo
- 컴파일러: MSVC (Microsoft Visual C++)
- 플랫폼: Windows (32/64비트)

## 개발 환경 설정

### 필수 요구사항
- Rust (1.86.0 이상)
- Visual Studio Build Tools (MSVC)

### IDE 설정 (선택사항)
#### Visual Studio Code
- rust-analyzer: Rust 언어 서버
- CodeLLDB: 디버깅 지원
- crates: Cargo.toml 의존성 관리
- Better TOML: TOML 파일 지원

### 빌드 타겟
- 64비트: `x86_64-pc-windows-msvc`
- 32비트: `i686-pc-windows-msvc`

## 프로젝트 구조
```
lidar_server/
├── src/                # 소스 코드
│   ├── main.rs         # 메인 진입점
│   ├── common/         # 공통 데이터
│   │   ├── data.rs
│   │   └── mod.rs
│   ├── udp/            # udp listener
│   │   ├── listener.rs
│   │   └── mod.rs
│   └── ws/             # websocket server
│   │   ├── server.rs 
│   │   └── mod.rs
├── tests/              # 테스트 코드
├── docs/               # 문서
└── Cargo.toml          # 프로젝트 설정
```

## 빌드 및 실행

### 개발 빌드
```bash
cargo build
```

### 릴리즈 빌드
```bash
cargo build --release
```

### 32비트 빌드
```bash
cargo build --target i686-pc-windows-msvc
```

## 라이선스
MIT License

## Commit Style
- feat: 기능 추가
- fix: 버그 수정
- docs: 문서 수정
- test: 테스트 코드
- refector: 리팩토링
- build: 빌드 파일 수정
- chore: 자잘한 수정
- rename: 파일명 변경
- remove: 파일 삭제
- release: 버전 릴리즈