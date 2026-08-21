use super::models::{PaceLevel, PaceStatus};

pub fn calculate_pace(
    used_percent: f64,
    total_period_seconds: i64,
    resets_in_seconds: i64,
) -> PaceStatus {
    if total_period_seconds <= 0 || resets_in_seconds <= 0 {
        return PaceStatus {
            level: PaceLevel::Unknown,
            projected_usage_percent: None,
            message: "无法评估配速（周期参数不足）".to_string(),
        };
    }

    // 限制在合理区间
    let used_percent = used_percent.clamp(0.0, 100.0);
    let resets_in_seconds = resets_in_seconds.min(total_period_seconds);
    let elapsed_seconds = (total_period_seconds - resets_in_seconds).max(1);

    // 计算每秒消耗百分比 (Burn rate)
    let burn_rate = used_percent / (elapsed_seconds as f64);
    let projected_remaining = burn_rate * (resets_in_seconds as f64);
    let projected_total = (used_percent + projected_remaining).round();

    let (level, message) = if used_percent >= 99.0 {
        (
            PaceLevel::OverPace,
            "配额已耗尽，请等待重置或切换模型".to_string(),
        )
    } else if projected_total <= 90.0 {
        (
            PaceLevel::OnPace,
            format!("使用节奏健康，预计周期结束时用量 {}%，可平稳过渡", projected_total),
        )
    } else if projected_total <= 105.0 {
        (
            PaceLevel::Tight,
            format!("用量较为紧凑，预计周期结束时用量 {}%，建议适当控制", projected_total),
        )
    } else {
        (
            PaceLevel::OverPace,
            format!("当前消耗过快，预计本周期将超标（约 {}%），建议切换备用模型", projected_total),
        )
    };

    PaceStatus {
        level,
        projected_usage_percent: Some(projected_total),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_on_pace_calculation() {
        // 5小时周期(18000s)，过了2.5小时(剩9000s)，用了30%
        let status = calculate_pace(30.0, 18000, 9000);
        assert_eq!(status.level, PaceLevel::OnPace);
        assert_eq!(status.projected_usage_percent, Some(60.0));
    }

    #[test]
    fn test_over_pace_calculation() {
        // 5小时周期(18000s)，过了1小时(剩14400s，耗时3600s)，已经用了40%
        // burn_rate = 40 / 3600 = 0.01111% / s
        // projected_remaining = 0.01111 * 14400 = 160%
        // total = 200%
        let status = calculate_pace(40.0, 18000, 14400);
        assert_eq!(status.level, PaceLevel::OverPace);
        assert_eq!(status.projected_usage_percent, Some(200.0));
    }
}
