# Tracing Log App

원격 서버에 SSH로 접속해 터미널 작업과 실시간 로그 추적을 함께 제공하는 Rust + Tauri 데스크톱 앱입니다.

기능 범위, 보안 원칙, 기술 구조 및 MVP 단계는 [기능 설계 문서](docs/functional-design.md)에서 확인할 수 있습니다.

## 현재 구현 상태

- Tauri 2 + React 기반 데스크톱 셸
- 접속 프로필의 로컬 저장용 Rust 명령 계층
- 환경별 서버 트리, 다중 세션 탭, SSH 세션 생성 대화상자
- 터미널·원격 파일·로그 패널을 포함한 서버 작업 영역 UI

## 실행

```powershell
pnpm install
pnpm tauri dev
```

현재 화면의 터미널·원격 파일·로그 데이터는 UI/스트림 동작을 검증하기 위한 로컬 데모 모드입니다. 실제 SSH 인증, known-hosts 및 원격 `tail -F` 전송 계층은 다음 구현 단위입니다.

## 문제 해결

Windows에서 Cargo가 생성한 `src-tauri/target` 내부 DLL은 빌드 중 잠길 수 있습니다. Vite 설정에서 이 폴더를 감시 대상에서 제외했으므로, 이전 실행이 남아 있다면 터미널을 종료한 뒤 `pnpm tauri dev`를 다시 실행하면 됩니다.
