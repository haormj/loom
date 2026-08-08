use contracts::{
    DeployProvider, DeploymentProviderCandidate, DeploymentProviderCandidateStatus,
    DeploymentProviderPolicy, DeploymentSourceModel,
};

use crate::{code_evidence::DeploymentCodeProbe, existing::ExistingDeploymentFiles};

#[derive(Debug, Clone)]
pub struct DeploymentStrategy {
    pub provider: DeployProvider,
    pub reason: String,
    pub policy: DeploymentProviderPolicy,
    pub candidates: Vec<DeploymentProviderCandidate>,
}

pub fn resolve_deployment_strategy(
    code_probe: &DeploymentCodeProbe,
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: Option<DeploymentProviderPolicy>,
) -> DeploymentStrategy {
    let policy = normalize_provider_policy(policy);
    let provider = select_provider(source_model, existing, &policy);
    let reason = reason_for(provider, code_probe, source_model, existing, &policy);
    let candidates = provider_candidates(provider, code_probe, source_model, existing, &policy);
    DeploymentStrategy {
        provider,
        reason,
        policy,
        candidates,
    }
}

pub fn normalize_provider_policy(
    policy: Option<DeploymentProviderPolicy>,
) -> DeploymentProviderPolicy {
    let Some(mut policy) = policy else {
        return DeploymentProviderPolicy::default();
    };
    if policy.force_generate {
        policy.provider = Some(DeployProvider::Generated);
        policy.reuse_existing = false;
    }
    policy
}

fn select_provider(
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: &DeploymentProviderPolicy,
) -> DeployProvider {
    if policy.force_generate {
        return DeployProvider::Generated;
    }
    if let Some(provider) = policy.provider {
        return provider;
    }
    if !policy.reuse_existing {
        return DeployProvider::Generated;
    }
    if existing.compose_path.is_some() {
        return DeployProvider::ComposeExisting;
    }
    if existing.dockerfile_path.is_some() && source_model.services.len() <= 1 {
        return DeployProvider::DockerfileExisting;
    }
    DeployProvider::Generated
}

fn reason_for(
    provider: DeployProvider,
    code_probe: &DeploymentCodeProbe,
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: &DeploymentProviderPolicy,
) -> String {
    if policy.force_generate {
        return "Provider policy 强制生成 Dockerfile/Compose 资产。".to_string();
    }
    if let Some(forced) = policy.provider {
        return format!("Provider policy 显式选择了 {}。", provider_label(forced));
    }
    if !policy.reuse_existing {
        return "Provider policy 禁用了现有部署资产复用。".to_string();
    }
    match provider {
        DeployProvider::ComposeExisting => {
            "根目录存在 Compose 文件，Loom 将在生成回退之前优先尝试它。".to_string()
        }
        DeployProvider::DockerfileExisting => {
            "根目录存在 Dockerfile，Loom 将复用它并生成 Compose 包装。".to_string()
        }
        DeployProvider::Generated => {
            if existing.dockerfile_path.is_some() && source_model.services.len() > 1 {
                "现有根 Dockerfile 无法表示多个应用服务；生成的部署资产更安全。".to_string()
            } else {
                format!(
                    "仓库探针发现 {:?} 运行时证据，Loom 将生成部署资产。",
                    code_probe.kind
                )
            }
        }
    }
}

fn provider_candidates(
    selected: DeployProvider,
    code_probe: &DeploymentCodeProbe,
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: &DeploymentProviderPolicy,
) -> Vec<DeploymentProviderCandidate> {
    [
        DeployProvider::ComposeExisting,
        DeployProvider::DockerfileExisting,
        DeployProvider::Generated,
    ]
    .into_iter()
    .map(|provider| DeploymentProviderCandidate {
        provider,
        status: candidate_status(provider, selected, source_model, existing, policy),
        reason: candidate_reason(provider, code_probe, source_model, existing, policy),
        commands: candidate_commands(provider),
    })
    .collect()
}

fn candidate_status(
    provider: DeployProvider,
    selected: DeployProvider,
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: &DeploymentProviderPolicy,
) -> DeploymentProviderCandidateStatus {
    if provider == selected {
        return DeploymentProviderCandidateStatus::Selected;
    }
    if policy.force_generate && provider != DeployProvider::Generated {
        return DeploymentProviderCandidateStatus::Skipped;
    }
    if policy.provider.is_some_and(|forced| forced != provider) {
        return DeploymentProviderCandidateStatus::Skipped;
    }
    match provider {
        DeployProvider::ComposeExisting => existing
            .compose_path
            .as_ref()
            .map(|_| DeploymentProviderCandidateStatus::Available)
            .unwrap_or(DeploymentProviderCandidateStatus::Skipped),
        DeployProvider::DockerfileExisting => {
            if existing.dockerfile_path.is_none() || source_model.services.len() > 1 {
                DeploymentProviderCandidateStatus::Skipped
            } else {
                DeploymentProviderCandidateStatus::Available
            }
        }
        DeployProvider::Generated => DeploymentProviderCandidateStatus::Available,
    }
}

fn candidate_reason(
    provider: DeployProvider,
    code_probe: &DeploymentCodeProbe,
    source_model: &DeploymentSourceModel,
    existing: &ExistingDeploymentFiles,
    policy: &DeploymentProviderPolicy,
) -> String {
    if policy.force_generate && provider != DeployProvider::Generated {
        return "因 Provider policy 强制生成部署资产而跳过。".to_string();
    }
    if let Some(forced) = policy.provider {
        if forced != provider {
            return format!(
                "因 Provider policy 显式选择了 {} 而跳过。",
                provider_label(forced)
            );
        }
    }
    match provider {
        DeployProvider::ComposeExisting => existing
            .compose_path
            .as_ref()
            .map(|_| "在部署根目录找到现有 Compose 文件。".to_string())
            .unwrap_or_else(|| "未找到根目录 Compose 文件。".to_string()),
        DeployProvider::DockerfileExisting => {
            if existing.dockerfile_path.is_none() {
                "未找到根目录 Dockerfile。".to_string()
            } else if source_model.services.len() > 1 {
                "因单个根 Dockerfile 无法表示多个应用服务而跳过。".to_string()
            } else {
                "在部署根目录找到现有 Dockerfile。".to_string()
            }
        }
        DeployProvider::Generated => format!(
            "可用，因为 Loom 可以将 {:?} 运行时证据建模为生成的本地部署资产。",
            code_probe.kind
        ),
    }
}

fn candidate_commands(provider: DeployProvider) -> Vec<Vec<String>> {
    match provider {
        DeployProvider::ComposeExisting => {
            vec![vec![
                "docker".to_string(),
                "compose".to_string(),
                "config".to_string(),
                "--quiet".to_string(),
            ]]
        }
        DeployProvider::DockerfileExisting | DeployProvider::Generated => vec![vec![
            "docker".to_string(),
            "compose".to_string(),
            "up".to_string(),
            "-d".to_string(),
            "--build".to_string(),
        ]],
    }
}

fn provider_label(provider: DeployProvider) -> &'static str {
    match provider {
        DeployProvider::ComposeExisting => "compose-existing",
        DeployProvider::DockerfileExisting => "dockerfile-existing",
        DeployProvider::Generated => "generated",
    }
}
