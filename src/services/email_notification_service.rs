use crate::repositories::notification_preferences_repository::NotificationPreferencesRepository;
/// Email Notification Service
///
/// Sends the monthly statistics summary email to all opted-in users.
///
/// Called by the `"0 8 2 * *"` cron trigger (8 AM UTC on day 2 of each month).
/// Emails are always sent regardless of activity level — zero-link users get a
/// "create your first link" nudge, zero-click users see their (empty) stats.
use crate::repositories::{AnalyticsRepository, LinkRepository, OrgRepository};
use crate::utils::email::{OrgMonthlySummary, TopLinkSummary, send_monthly_stats_email};
use crate::utils::{get_frontend_url, is_mailgun_configured};
use chrono::{Datelike, Utc};
use worker::d1::D1Database;
use worker::{Env, console_error, console_log, console_warn};

/// Format a year/month pair as a "YYYY-MM" string for the counter table.
fn year_month_label(year: i32, month: u32) -> String {
    format!("{:04}-{:02}", year, month)
}

/// Format a year/month pair as a human-readable label, e.g. "May 2026".
fn month_label(year: i32, month: u32) -> String {
    let month_name = match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    };
    format!("{} {}", month_name, year)
}

/// Send the monthly statistics email to every opted-in, non-suspended user.
///
/// Returns `(sent, errors)` counts.
pub async fn send_monthly_stats_to_all_users(db: &D1Database, env: &Env) -> (usize, usize) {
    // Guard: only proceed when Mailgun is configured
    if !is_mailgun_configured(env) {
        console_log!("[email_notifications] Mailgun not configured — skipping monthly stats job");
        return (0, 0);
    }

    let now = Utc::now();
    let (prev_year, prev_month) = if now.month() == 1 {
        (now.year() - 1, 12u32)
    } else {
        (now.year(), now.month() - 1)
    };
    let (prev_prev_year, prev_prev_month) = if prev_month == 1 {
        (prev_year - 1, 12u32)
    } else {
        (prev_year, prev_month - 1)
    };
    let label = month_label(prev_year, prev_month);
    let prev_year_month = year_month_label(prev_year, prev_month);
    let prev_prev_year_month = year_month_label(prev_prev_year, prev_prev_month);
    let frontend_url = get_frontend_url(env);

    let prefs_repo = NotificationPreferencesRepository::new();
    let org_repo = OrgRepository::new();
    let link_repo = LinkRepository::new();
    let analytics_repo = AnalyticsRepository::new();

    // Fetch all opted-in, non-suspended users
    let recipients = match prefs_repo.get_monthly_stats_recipients(db).await {
        Ok(r) => r,
        Err(e) => {
            console_error!("[email_notifications] Failed to fetch recipients: {}", e);
            return (0, 1);
        }
    };

    console_log!(
        "[email_notifications] Sending monthly stats ({}) to {} recipient(s)",
        label,
        recipients.len()
    );

    let mut sent = 0usize;
    let mut errors = 0usize;

    for user in &recipients {
        // Gather all orgs this user owns
        let all_orgs = match org_repo.get_user_orgs(db, &user.user_id).await {
            Ok(orgs) => orgs,
            Err(e) => {
                console_warn!(
                    "[email_notifications] Failed to fetch orgs for user {}: {}",
                    user.user_id,
                    e
                );
                errors += 1;
                continue;
            }
        };

        // Only include orgs where the user is the owner
        let owned_orgs: Vec<_> = all_orgs.into_iter().filter(|o| o.role == "owner").collect();

        if owned_orgs.is_empty() {
            // No owned orgs — nothing meaningful to email about, skip silently
            continue;
        }

        // Build per-org summaries
        let mut org_summaries: Vec<OrgMonthlySummary> = Vec::new();

        for org in &owned_orgs {
            // Count active links
            let active_links = match link_repo.get_dashboard_stats(db, &org.id).await {
                Ok(stats) => stats.active_links,
                Err(e) => {
                    console_warn!(
                        "[email_notifications] Failed to get link stats for org {}: {}",
                        org.id,
                        e
                    );
                    0
                }
            };

            // Total clicks in the previous month (from pre-aggregated counter)
            let total_clicks = match analytics_repo
                .get_org_monthly_clicks(db, &org.id, &prev_year_month)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    console_warn!(
                        "[email_notifications] Failed to get clicks for org {}: {}",
                        org.id,
                        e
                    );
                    0
                }
            };

            // Comparison: clicks in the month before the previous one
            let prev_month_clicks = match analytics_repo
                .get_org_monthly_clicks(db, &org.id, &prev_prev_year_month)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    console_warn!(
                        "[email_notifications] Failed to get prev-prev clicks for org {}: {}",
                        org.id,
                        e
                    );
                    0
                }
            };

            // Top links — only fetched when there were clicks
            let top_links = if total_clicks > 0 {
                match analytics_repo
                    .get_org_monthly_top_links(db, &org.id, &prev_year_month, 5)
                    .await
                {
                    Ok(links) => links
                        .into_iter()
                        .map(|l| TopLinkSummary {
                            short_code: l.short_code,
                            title: l.title,
                            clicks: l.count,
                        })
                        .collect(),
                    Err(e) => {
                        console_warn!(
                            "[email_notifications] Failed to get top links for org {}: {}",
                            org.id,
                            e
                        );
                        vec![]
                    }
                }
            } else {
                vec![]
            };

            org_summaries.push(OrgMonthlySummary {
                org_name: org.name.clone(),
                total_links: active_links,
                total_clicks,
                prev_month_clicks,
                top_links,
            });
        }

        // Send the email — always, even if all orgs have zero activity
        match send_monthly_stats_email(
            env,
            &user.email,
            user.name.as_deref(),
            &label,
            &org_summaries,
            &frontend_url,
        )
        .await
        {
            Ok(()) => {
                console_log!(
                    "[email_notifications] Sent monthly stats to {} ({})",
                    user.email,
                    user.user_id
                );
                sent += 1;
            }
            Err(e) => {
                console_error!(
                    "[email_notifications] Failed to send to {} ({}): {}",
                    user.email,
                    user.user_id,
                    e
                );
                errors += 1;
            }
        }
    }

    console_log!(
        "[email_notifications] Monthly stats job complete: {} sent, {} error(s)",
        sent,
        errors
    );
    (sent, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_label() {
        assert_eq!(month_label(2026, 5), "May 2026");
        assert_eq!(month_label(2025, 12), "December 2025");
        assert_eq!(month_label(2026, 1), "January 2026");
    }

    #[test]
    fn test_year_month_label() {
        assert_eq!(year_month_label(2026, 5), "2026-05");
        assert_eq!(year_month_label(2025, 12), "2025-12");
        assert_eq!(year_month_label(2026, 1), "2026-01");
        assert_eq!(year_month_label(2024, 2), "2024-02");
    }

    #[test]
    fn test_month_label_all_twelve_months() {
        let expected = [
            (1, "January"),
            (2, "February"),
            (3, "March"),
            (4, "April"),
            (5, "May"),
            (6, "June"),
            (7, "July"),
            (8, "August"),
            (9, "September"),
            (10, "October"),
            (11, "November"),
            (12, "December"),
        ];
        for (m, name) in expected {
            assert_eq!(
                month_label(2026, m),
                format!("{name} 2026"),
                "failed for month {m}"
            );
        }
    }

    #[test]
    fn test_month_label_invalid_month_does_not_panic() {
        // Month 0 and 13 hit the _ => "Unknown" arm — must not panic
        assert_eq!(month_label(2026, 0), "Unknown 2026");
        assert_eq!(month_label(2026, 13), "Unknown 2026");
    }
}
