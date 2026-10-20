use std::collections::HashSet;

use crate::catalog::PackageCatalog;
use crate::package::Package;

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyOperator {
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub operator: Option<DependencyOperator>,
    pub version: Option<String>,
}

impl Dependency {
    pub fn parse(expression: &str) -> anyhow::Result<Self> {
        let expression = expression.trim();

        if expression.is_empty() {
            anyhow::bail!("empty dependency expression");
        }

        let operators = [
            (">=", DependencyOperator::GreaterEqual),
            ("<=", DependencyOperator::LessEqual),
            ("=", DependencyOperator::Equal),
            (">", DependencyOperator::Greater),
            ("<", DependencyOperator::Less),
        ];

        for (symbol, operator) in operators {
            if let Some((name, version)) = expression.split_once(symbol) {
                let name = name.trim();
                let version = version.trim();

                if name.is_empty() {
                    anyhow::bail!("invalid dependency expression: {expression}");
                }

                if version.is_empty() {
                    anyhow::bail!("dependency {expression} has no version");
                }

                return Ok(Self {
                    name: name.to_string(),
                    operator: Some(operator),
                    version: Some(version.to_string()),
                });
            }
        }

        Ok(Self {
            name: expression.to_string(),
            operator: None,
            version: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Capability {
    name: String,
    version: Option<String>,
}

impl Capability {
    fn parse(expression: &str) -> anyhow::Result<Self> {
        let expression = expression.trim();

        if expression.is_empty() {
            anyhow::bail!("empty provided capability");
        }

        if let Some((name, version)) = expression.split_once('=') {
            let name = name.trim();
            let version = version.trim();

            if name.is_empty() || version.is_empty() {
                anyhow::bail!("invalid provided capability: {expression}");
            }

            return Ok(Self {
                name: name.to_string(),
                version: Some(version.to_string()),
            });
        }

        Ok(Self {
            name: expression.to_string(),
            version: None,
        })
    }
}

pub struct DependencyResolver<'a> {
    catalog: &'a mut PackageCatalog,
    visiting: HashSet<String>,
    resolved: HashSet<String>,
    order: Vec<Package>,
}

impl<'a> DependencyResolver<'a> {
    pub fn new(catalog: &'a mut PackageCatalog) -> Self {
        Self {
            catalog,
            visiting: HashSet::new(),
            resolved: HashSet::new(),
            order: Vec::new(),
        }
    }

    pub fn resolve(&mut self, requested: &[String]) -> anyhow::Result<InstallPlan> {
        for expression in requested {
            self.visit(expression)?;
        }

        Ok(InstallPlan {
            packages: self.order.clone(),
        })
    }

    fn visit(&mut self, expression: &str) -> anyhow::Result<()> {
        let dependency = Dependency::parse(expression)?;

        let package = self.find_satisfying_package(&dependency)?;

        let package_id = package.id();

        if self.resolved.contains(&package_id) {
            return Ok(());
        }

        if !self.visiting.insert(package_id.clone()) {
            anyhow::bail!("dependency cycle detected involving {}", dependency.name);
        }

        for expression in &package.dependencies {
            self.visit(expression)?;
        }

        self.visiting.remove(&package_id);
        self.resolved.insert(package_id);
        self.order.push(package);

        Ok(())
    }

    fn find_satisfying_package(&mut self, dependency: &Dependency) -> anyhow::Result<Package> {
        // First try a real package with this name.
        if let Some(package) = self.catalog.find(&dependency.name)? {
            if package_satisfies_dependency(&package, dependency)? {
                return Ok(package);
            }
        }

        // Then look for packages providing this capability.
        let providers = self.catalog.find_providers(&dependency.name)?;

        for package in providers {
            if package_provides_dependency(&package, dependency)? {
                return Ok(package);
            }
        }

        anyhow::bail!(
            "no package provides {}{}",
            dependency.name,
            format_requirement(dependency)
        );
    }
}

fn package_satisfies_dependency(
    package: &Package,
    dependency: &Dependency,
) -> anyhow::Result<bool> {
    let operator = match dependency.operator {
        Some(operator) => operator,
        None => return Ok(true),
    };

    let required_version = dependency
        .version
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("missing dependency version"))?;

    if package.version.is_empty() {
        return Ok(false);
    }

    compare_versions(&package.version, required_version, operator)
}

fn package_provides_dependency(package: &Package, dependency: &Dependency) -> anyhow::Result<bool> {
    for expression in &package.provides {
        let capability = Capability::parse(expression)?;

        if capability.name != dependency.name {
            continue;
        }

        match (
            dependency.operator,
            dependency.version.as_deref(),
            capability.version.as_deref(),
        ) {
            // Example:
            //
            // dependency: sh
            // provider: sh
            //
            // Any matching unversioned capability is enough.
            (None, None, _) => {
                return Ok(true);
            }

            // Example:
            //
            // dependency: libncursesw.so=6-64
            // provider: libncursesw.so=6-64
            //
            // Compare the capability version, NOT the
            // package version.
            (Some(operator), Some(required), Some(provided)) => {
                if compare_versions(provided, required, operator)? {
                    return Ok(true);
                }
            }

            // A versioned dependency cannot be satisfied
            // by an unversioned capability.
            _ => {}
        }
    }

    Ok(false)
}

fn compare_versions(
    actual: &str,
    required: &str,
    operator: DependencyOperator,
) -> anyhow::Result<bool> {
    // Temporary comparison layer.
    //
    // Arch's real version ordering is more complicated than
    // normal string comparison. This function is deliberately
    // isolated so it can later be replaced with an
    // Arch-compatible version comparator.
    let ordering = actual.cmp(required);

    Ok(match operator {
        DependencyOperator::Equal => ordering.is_eq(),
        DependencyOperator::Greater => ordering.is_gt(),
        DependencyOperator::GreaterEqual => ordering.is_ge(),
        DependencyOperator::Less => ordering.is_lt(),
        DependencyOperator::LessEqual => ordering.is_le(),
    })
}

fn format_requirement(dependency: &Dependency) -> String {
    match (&dependency.operator, &dependency.version) {
        (Some(operator), Some(version)) => {
            let symbol = match operator {
                DependencyOperator::Equal => "=",
                DependencyOperator::Greater => ">",
                DependencyOperator::GreaterEqual => ">=",
                DependencyOperator::Less => "<",
                DependencyOperator::LessEqual => "<=",
            };

            format!("{symbol}{version}")
        }

        _ => String::new(),
    }
}
