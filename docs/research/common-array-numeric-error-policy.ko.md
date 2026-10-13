# RustJ 공통배열대수: J 비교 허용 오차와 최적화 수치 오차 계약

상태: **정책 초안 + 분석 전용 수치 사전검사 구현**. J의 동적 CCT 및 Fit 실행 구현, 재결합 최적화, 실제 EqGraph E-Class 병합, 수치 오차 보장, 전체 J 관찰 동치는 **미완료**.

## 1. 서로 다른 세 층

| 층 | 의미 | 관리 위치 | 합법성 결론 |
|---|---|---|---|
| **J 언어 의미론** | `=`, `-:`, 검색 등 해당 primitive의 비교 결과. 기본 허용 오차, `!.t`, `9!:18`/`9!:19`와 현재 실행 상태가 결정한다. | J Semantic/Runtime, `comparison_policy`; 호출별 정책 snapshot/버전이 필요 | 정확히 **J의 비교 결과**를 산출해야 한다 |
| **공통배열대수의 수학적 관계** | shape, 축, 접근 관계 및 적절한 도메인에서 수학적으로 성립하는 항등식 | Graph/rewrite proof + 전제조건 | **부동소수점 재평가의 J 관찰 동치**는 별도 증명 |
| **최적화 수치 오차 예산** | 명시적 오차 척도·상한·전파 증명. 기본 `Exact`; 허용치를 자동 부여하지 않음 | 후속 수치 최적화 후보/검증 단계 | 예산 만족만으로 J 코드 재작성, EqGraph 병합 또는 오류/효과 제거 불가 |

**J 비교 허용 오차와 최적화 예산은 다른 정책이다.** J 기본값에서 어떤 `a = b`가 1이 되더라도 그 두 산출물을 공통배열대수의 등식 클래스에 합칠 수 없다. 근사 비교는 **추이적이지 않다**.

## 2. J 비교 의미론의 정확한 기준

J Dictionary는 일반적인 유한 실수 비교를 다음 식으로 설명한다.

```text
t_default = 2^-44 ≈ 5.684341886080802e-14
abs(a - b) <= t * max(abs(a), abs(b))
```

그러나 실제 `jsrc/vcomp.h::TCMPEQ`는 C의 반올림을 거치는 분기형 식이다. 기본 실행에서 `cct = 1 - 2^-44`라는 내부 표현을 사용하며, RustJ의 현재 기본 비교 스냅샷은 이 **C 식**을 따른다. 따라서 문서의 실수 수학식만 `f64`로 직접 계산해 경계값을 대체하면 안 된다. `0` 근처, subnormal, `+0/-0`, 무한대, NaN, 양·음 값, ULP 인접 경계는 독립적으로 확인한다.

- `9!:18 ''` 조회 및 `9!:19` 설정은 **변경 가능한 전역/실행 문맥 의미론**이다. 정책 값·버전·가드가 필요한데 **현재 RustJ 실행기에는 구현되지 않았다**.
- `=!.0`은 허용 오차 0의 **J 엄격 비교**를 요구한다. 비트 동일성(예: `+0/-0`)과 혼동하지 않는다. 일반 `!.t` 유도·호출별 정책 역시 **미구현**.
- `-:` Match, 정렬/검색/중복 제거 등 관여 primitive마다 jsource의 적용 범위, 타입/shape/boxed/sparse/rank 및 오류 우선순위를 별도 점검한다. `=` 지원을 모든 연산에 일반화하지 않는다.
- `jsource/test/g421e.ijs`는 `+/` 합산 순서 차이를 고려하여 일시적으로 CCT를 조정한 회귀 사례이다. 이는 **허용 오차를 둔 비교 테스트의 사례**이지, `+/` 재결합의 J 관찰 동치나 모든 최적화에 쓰는 예산을 뜻하지 않는다.

## 3. EqGraph / SPORES / 재작성 게이트

`src/numeric_error_policy.rs`에는 `NumericEqualityClaim`, `OptimizationErrorBudget`와 `eqgraph_numeric_precheck`를 추가한다.

- 기본 예산은 `Exact`이다. `Bounded { max_absolute, max_relative }`는 두 유한·비음수 수치를 **명시적으로** 입력할 때만 생성한다. 이는 아직 실행 정책이나 오차 전파 정리를 구성하지 않는다.
- `CoordinateIdentity`, `RealAlgebraicIdentity`만으로는 E-Class union을 거부한다.
- `Bounded` 요청은 J의 비교 허용 오차 수치와 같더라도 E-Class union을 거부한다.
- `ExactJObservational + Exact`도 **수치 전제 조건을 통과할 뿐** Rank/Cell/Frame, NAME, effect, 첫 오류, prototype/fill, 자료형, 평가 순서, CCT/Fit 상태, 실제 수치값의 동치 등 남은 증명 없이는 union을 허용하지 않는다.
- 근사 결과는 필요하다면 향후 **비등식 후보 관계/비용 탐색**으로 관리하되 정확 동치 클래스와 절대 섞지 않는다.

## 4. 검증·후속 체크리스트

| Test / task | Purpose | Guarantee | Failure meaning | Status |
|---|---|---|---|---|
| `optimization_error_budget_defaults_to_exact_and_rejects_invalid_proposals` | 명시적 예산 정책 기본값·입력 검사 | 기본 `Exact`; NaN/무한/음수/무의미한 zero-bounded 거부 | J CCT가 최적화 예산으로 무단 승계 | **PR CI 대기** |
| `j_tolerant_comparison_and_real_algebra_do_not_admit_eqgraph_union` | 비추이 J 비교와 E-Class 수치 사전검사 분리 | 비교 A~B, B~C라도 A~C 불성립; 수학적 항등·유계 오차만으로 union 금지 | 비추이 비교의 등식 폐포 또는 허용 예산 오용 | **PR CI 대기** |
| `9!:18` / `9!:19` | CCT 조회·변경·스냅샷 버전·범위 검증, 캐시 무효화 | 같은 실행 문맥의 J 의미론 보존, 설정 변경 후 결과 반영 | 고정 기본 CCT를 동적 J 의미론으로 오인 | **미구현 / CI 미검증** |
| `=!.0`, 일반 `!.t`, `-:` 및 의존 primitive | 핀된 jsource 실제 J-C 오라클과 교차 검증 | 유한 경계·NaN/Inf/±0·타입/Rank·오류 우선순위까지 확인 | 수학적 공식만으로 J 실제 구현 차이를 숨김 | **부분 구현 / 추가 오라클 미검증** |
| 재결합·병렬 Reduce 오차 인증 | 연산별 입력 범위/NaN/overflow/underflow, ULP·abs·rel 척도와 누적/관찰 평가 정책 | **도메인에 의존하는 수치 보증**과 J 관찰 의미론 별도 판정 | `2^-44`를 임의 재결합 허용치로 오해 | **미구현** |
| egg / EqGraph 연결 | 정확 J 동치 증거와 의미론적 효과/오류/숫자 정책의 독립 검증 | 비추이 근사 비교를 `union`의 동일성 근거로 쓰지 않음 | 거짓 e-class 동치, 역전파로 무효 재작성 | **미연결 / 미검증** |

## 5. 근거

- [J Dictionary: Equal `=`](https://www.jsoftware.com/help/dictionary/d000.htm)
- [J Dictionary: Global Parameters `9!:`](https://jsoftware.com/help/dictionary/dx009.htm)
- [J Dictionary: Fit `!.`](https://jsoftware.com/help/dictionary/d411.htm)
- [고정 비교 근거: jsource `jsrc/vcomp.h`](https://github.com/jsoftware/jsource/blob/13994ffa1ed5f06f79fad6e9822a7ed2d29b1528/jsrc/vcomp.h), `jsrc/xa.c`의 CCT 관리, `jsource/test/g421e.ijs`의 `+/` 검증
- RustJ: `src/comparison_policy.rs`, `src/kernels.rs`, `src/index_ops.rs`, `tools/tolerance_audit.py`. 현재 `comparison_policy`는 **기본 CCT만** 구현한다.

**승격 금지:** 정책 사전검사와 단위 테스트가 통과해도 전체 J 비교 동치·수치 오차 보증·EqGraph union·실행 최적화를 완료로 표시하지 않는다.
