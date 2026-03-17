# Release Setup Guide

## 코드 서명 없이 배포 (기본)

코드 서명 secrets가 없어도 워크플로우는 정상 동작합니다.
- Universal Binary는 ad-hoc 서명으로 생성됨
- 사용자가 "확인되지 않은 개발자" 경고를 수동으로 허용해야 함

## Apple 코드 서명 + 공증 설정

"확인되지 않은 개발자" 경고를 제거하려면 아래 secrets를 설정하세요.

### 사전 요구사항

- [Apple Developer Program](https://developer.apple.com/programs/) 가입 ($99/년)
- Xcode 또는 Keychain Access에서 **Developer ID Application** 인증서 생성

### GitHub Secrets 설정

Repository → Settings → Secrets and variables → Actions에서 추가:

| Secret | 설명 | 얻는 방법 |
|---|---|---|
| `APPLE_CERTIFICATE` | Base64 인코딩된 .p12 인증서 | `base64 -i certificate.p12 \| pbcopy` |
| `APPLE_CERTIFICATE_PASSWORD` | .p12 내보내기 시 설정한 비밀번호 | 인증서 내보내기 시 입력한 값 |
| `APPLE_SIGNING_IDENTITY` | 서명 ID | `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_ID` | Apple ID 이메일 | 개발자 계정 이메일 |
| `APPLE_PASSWORD` | App-Specific Password | [appleid.apple.com](https://appleid.apple.com) → App-Specific Passwords |
| `APPLE_TEAM_ID` | 10자리 팀 ID | developer.apple.com → Membership → Team ID |
| `KEYCHAIN_PASSWORD` | CI 임시 키체인 비밀번호 | 아무 랜덤 문자열 (예: `openssl rand -base64 32`) |

### 인증서 생성 절차

```bash
# 1. Keychain Access에서 Developer ID Application 인증서를 .p12로 내보내기
#    또는 CLI로:
security find-identity -v -p codesigning

# 2. Base64 인코딩
base64 -i ~/Desktop/certificate.p12 | pbcopy

# 3. GitHub Secret에 붙여넣기
```

## 릴리스 방법

```bash
# 버전 태그 생성 및 push
git tag v1.0.0
git push origin v1.0.0
```

워크플로우가 자동으로:
1. arm64 + x86_64 각각 빌드
2. Universal Binary 생성 (lipo)
3. Changelog 자동 생성 (feat/fix/other 분류)
4. Draft Release 생성 + DMG 첨부

Draft Release를 확인 후 Publish하면 배포 완료.

## 릴리스 산출물

| 파일 | 설명 |
|---|---|
| `MacDirStat_<version>_universal.dmg` | Universal Binary (Apple Silicon + Intel) |
| `MacDirStat_<version>_aarch64.dmg` | Apple Silicon 전용 |
| `MacDirStat_<version>_x64.dmg` | Intel 전용 |
| `MacDirStat_universal.app.tar.gz` | Tauri 자동 업데이트용 |
