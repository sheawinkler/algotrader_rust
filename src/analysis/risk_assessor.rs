use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Risk assessment for a trading opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub token_address: String,
    pub risk_score: f64, // 0-100, higher is riskier
    pub risk_factors: Vec<RiskFactor>,
    pub confidence: f64, // 0-1, confidence in the assessment
    pub last_updated: chrono::DateTime<Utc>,
}

/// Individual risk factors contributing to the overall risk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskFactor {
    HighDevHolding(f64),         // % of supply held by devs
    HighInsiderHolding(f64),     // % of supply held by insiders
    LowLiquidity(f64),           // Liquidity in SOL
    NewToken(u64),               // Age in days
    SuspiciousTokenAccount,      // Token account has suspicious characteristics
    HighVolatility(f64),         // 30-day volatility
    ConcentratedHolders(usize),  // Number of holders controlling >50% of supply
    RecentLargeTransfers(usize), // Number of large transfers in last 24h
    UnverifiedProgram,           // Token program is not verified
    NoLiquidityLock,             // No locked liquidity
    HighSellPressure(f64),       // % of sells in last 24h
    LowHolderCount(usize),       // Total number of holders
    SuspiciousCreator,           // Creator has suspicious history
}

impl RiskFactor {
    /// Get the risk score contribution of this factor
    pub fn score_contribution(&self) -> f64 {
        match self {
            | RiskFactor::HighDevHolding(pct) => {
                if *pct > 50.0 {
                    80.0
                } else if *pct > 30.0 {
                    60.0
                } else if *pct > 15.0 {
                    30.0
                } else {
                    0.0
                }
            }
            | RiskFactor::HighInsiderHolding(pct) => {
                if *pct > 70.0 {
                    90.0
                } else if *pct > 50.0 {
                    70.0
                } else if *pct > 30.0 {
                    40.0
                } else {
                    0.0
                }
            }
            | RiskFactor::LowLiquidity(sol) => {
                if *sol < 10_000.0 {
                    80.0
                } else if *sol < 50_000.0 {
                    50.0
                } else if *sol < 100_000.0 {
                    20.0
                } else {
                    0.0
                }
            }
            | RiskFactor::NewToken(days) => {
                if *days < 1 {
                    70.0
                } else if *days < 7 {
                    40.0
                } else if *days < 30 {
                    20.0
                } else {
                    0.0
                }
            }
            | RiskFactor::SuspiciousTokenAccount => 60.0,
            | RiskFactor::HighVolatility(vol) => {
                if *vol > 1.0 {
                    50.0
                } else if *vol > 0.5 {
                    30.0
                } else {
                    0.0
                }
            }
            | RiskFactor::ConcentratedHolders(count) => {
                if *count <= 3 {
                    80.0
                } else if *count <= 10 {
                    50.0
                } else if *count <= 25 {
                    20.0
                } else {
                    0.0
                }
            }
            | RiskFactor::RecentLargeTransfers(count) => {
                if *count > 10 {
                    70.0
                } else if *count > 5 {
                    40.0
                } else if *count > 1 {
                    20.0
                } else {
                    0.0
                }
            }
            | RiskFactor::UnverifiedProgram => 60.0,
            | RiskFactor::NoLiquidityLock => 50.0,
            | RiskFactor::HighSellPressure(pct) => {
                if *pct > 70.0 {
                    80.0
                } else if *pct > 50.0 {
                    50.0
                } else if *pct > 30.0 {
                    20.0
                } else {
                    0.0
                }
            }
            | RiskFactor::LowHolderCount(count) => {
                if *count < 100 {
                    60.0
                } else if *count < 500 {
                    30.0
                } else {
                    0.0
                }
            }
            | RiskFactor::SuspiciousCreator => 70.0,
        }
    }

    /// Get a human-readable description of the risk factor
    pub fn description(&self) -> String {
        match self {
            | RiskFactor::HighDevHolding(pct) => format!("High dev holding: {:.1}% of supply", pct),
            | RiskFactor::HighInsiderHolding(pct) => {
                format!("High insider holding: {:.1}% of supply", pct)
            }
            | RiskFactor::LowLiquidity(sol) => format!("Low liquidity: {:.1} SOL", sol),
            | RiskFactor::NewToken(days) => format!("New token: {} days old", days),
            | RiskFactor::SuspiciousTokenAccount => "Suspicious token account".to_string(),
            | RiskFactor::HighVolatility(vol) => format!("High volatility: {:.2} std dev", vol),
            | RiskFactor::ConcentratedHolders(count) => {
                format!("Concentrated holders: {} control >50%", count)
            }
            | RiskFactor::RecentLargeTransfers(count) => {
                format!("{} large transfers in 24h", count)
            }
            | RiskFactor::UnverifiedProgram => "Unverified token program".to_string(),
            | RiskFactor::NoLiquidityLock => "No locked liquidity".to_string(),
            | RiskFactor::HighSellPressure(pct) => {
                format!("High sell pressure: {:.1}% of volume", pct)
            }
            | RiskFactor::LowHolderCount(count) => format!("Low holder count: {}", count),
            | RiskFactor::SuspiciousCreator => "Suspicious creator history".to_string(),
        }
    }
}

/// Risk assessment configuration
#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_risk_score: f64,          // Maximum allowed risk score (0-100)
    pub min_liquidity_sol: f64,       // Minimum liquidity in SOL
    pub max_dev_holding_pct: f64,     // Maximum % of supply held by devs
    pub max_insider_holding_pct: f64, // Maximum % of supply held by insiders
    pub min_holder_count: usize,      // Minimum number of holders
    pub max_volatility: f64,          // Maximum allowed 30-day volatility
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_risk_score: 70.0,
            min_liquidity_sol: 50_000.0,
            max_dev_holding_pct: 30.0,
            max_insider_holding_pct: 50.0,
            min_holder_count: 100,
            max_volatility: 1.0,
        }
    }
}

/// Risk assessor for evaluating trading opportunities
pub struct RiskAssessor {
    config: RiskConfig,
    cached_assessments: HashMap<String, (RiskAssessment, chrono::DateTime<Utc>)>,
    cache_ttl: chrono::Duration, // How long to cache assessments
}

impl RiskAssessor {
    /// Create a new RiskAssessor with default configuration
    pub fn new() -> Self {
        Self {
            config: RiskConfig::default(),
            cached_assessments: HashMap::new(),
            cache_ttl: chrono::Duration::hours(1),
        }
    }

    /// Create a new RiskAssessor with custom configuration
    pub fn with_config(config: RiskConfig) -> Self {
        Self { config, cached_assessments: HashMap::new(), cache_ttl: chrono::Duration::hours(1) }
    }

    /// Assess the risk of a token
    pub async fn assess_token(&mut self, token_address: &str) -> Result<RiskAssessment> {
        // Check cache first
        if let Some((cached, timestamp)) = self.cached_assessments.get(token_address) {
            if Utc::now() - *timestamp < self.cache_ttl {
                return Ok(cached.clone());
            }
        }

        // TODO: Fetch actual token data from blockchain and DEXs
        // For now, we'll return a placeholder assessment

        let risk_factors = vec![
            RiskFactor::NewToken(2),            // Example: token is 2 days old
            RiskFactor::LowLiquidity(25_000.0), // Example: 25k SOL liquidity
            RiskFactor::ConcentratedHolders(5), // Example: 5 wallets control >50%
        ];

        let risk_score = self.calculate_risk_score(&risk_factors);

        let assessment = RiskAssessment {
            token_address: token_address.to_string(),
            risk_score,
            risk_factors,
            confidence: 0.85, // Example confidence
            last_updated: Utc::now(),
        };

        // Cache the assessment
        self.cached_assessments
            .insert(token_address.to_string(), (assessment.clone(), Utc::now()));

        Ok(assessment)
    }

    /// Calculate overall risk score from risk factors
    fn calculate_risk_score(&self, factors: &[RiskFactor]) -> f64 {
        if factors.is_empty() {
            return 0.0;
        }

        // Simple average of all risk factors for now
        // Could be weighted based on severity in the future
        let total: f64 = factors.iter().map(|f| f.score_contribution()).sum();

        (total / factors.len() as f64).min(100.0)
    }

    /// Check if a token passes our risk criteria
    pub fn is_acceptable_risk(&self, assessment: &RiskAssessment) -> bool {
        assessment.risk_score <= self.config.max_risk_score && assessment.confidence >= 0.7
    }

    /// Get a human-readable risk level
    pub fn risk_level(score: f64) -> &'static str {
        match score {
            | s if s < 20.0 => "Very Low",
            | s if s < 40.0 => "Low",
            | s if s < 60.0 => "Medium",
            | s if s < 80.0 => "High",
            | _ => "Very High",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_factor_scoring() {
        let high_dev = RiskFactor::HighDevHolding(60.0);
        assert!(high_dev.score_contribution() > 50.0);

        let low_liquidity = RiskFactor::LowLiquidity(5_000.0);
        assert!(low_liquidity.score_contribution() > 50.0);
    }

    #[tokio::test]
    async fn test_risk_assessment() {
        let mut assessor = RiskAssessor::new();
        let assessment = assessor.assess_token("test_token").await.unwrap();

        assert!(!assessment.risk_factors.is_empty());
        assert!(assessment.risk_score > 0.0);
        assert!(assessment.confidence > 0.0);
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(RiskAssessor::risk_level(15.0), "Very Low");
        assert_eq!(RiskAssessor::risk_level(35.0), "Low");
        assert_eq!(RiskAssessor::risk_level(55.0), "Medium");
        assert_eq!(RiskAssessor::risk_level(75.0), "High");
        assert_eq!(RiskAssessor::risk_level(95.0), "Very High");
    }
}
