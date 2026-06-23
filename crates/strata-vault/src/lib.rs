use serde::{Deserialize, Serialize};
use strata_core::{
    Cell, CellValue, Dim, DimTree, Edge, Graph, Member, Node, NodeId, NodePayload, Para, Rel, Section, Table, Term, Value, Scalar, Code, List, Anchor, Inline, EmphKind, MathBlock
};
use std::collections::HashMap;

/// YAMLで手書きするドキュメントのトップレベル表現
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlDocument {
    Resume(Box<YamlResume>),
    WorkHistory(Box<YamlWorkHistory>),
    Standard(YamlNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlResumeName {
    String(String),
    Struct {
        family: String,
        given: String,
    }
}

impl std::fmt::Display for YamlResumeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlResumeName::String(s) => write!(f, "{}", s),
            YamlResumeName::Struct { family, given } => write!(f, "{}　{}", family, given),
        }
    }
}

/// 人間用のシンプルな履歴書スキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResume {
    #[serde(rename = "type")]
    pub doc_type: String, // "resume"
    pub title: Option<String>,
    pub as_of: Option<String>,
    pub name: YamlResumeName,
    pub name_yomi: YamlResumeName,
    pub birthday: String,
    pub age: u32,
    pub photo: Option<String>,
    pub contact: YamlResumeContact,
    
    // 旧スキーマ用
    pub history: Option<Vec<YamlResumeHistory>>,
    pub certifications: Option<Vec<YamlResumeCert>>,
    pub motivation: Option<String>,

    // 新スキーマ用
    pub education: Option<Vec<YamlResumeEducation>>,
    pub work_history: Option<Vec<YamlResumeWork>>,
    pub licenses: Option<Vec<YamlResumeLicense>>,
    pub health: Option<String>,
    pub dependents_excluding_spouse: Option<u32>,
    pub hobbies: Option<String>,
    pub sports: Option<String>,
    pub spouse: Option<YamlResumeSpouse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeContact {
    pub zip: String,
    pub address: String,
    pub address_yomi: String,
    pub tel: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeHistory {
    pub period: String,
    pub org: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeCert {
    pub date: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeEducation {
    pub year: u32,
    pub month: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeWork {
    #[serde(default)]
    pub year: Option<u32>,
    #[serde(default)]
    pub month: Option<u32>,
    #[serde(default)]
    pub org: Option<String>,
    #[serde(default)]
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeLicense {
    pub year: u32,
    pub month: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlResumeSpouse {
    pub exists: bool,
    pub supported: bool,
}

/// 人間用のシンプルな職務経歴書スキーマ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlWorkHistory {
    #[serde(rename = "type")]
    pub doc_type: String, // "work_history"
    pub name: String,
    pub title: String,
    pub summary: String,
    pub tech_stack: Vec<YamlTechStackEntry>,
    pub projects: Vec<YamlProjectEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlTechStackEntry {
    pub category: String,
    pub label: String,
    pub tech: String,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProjectEntry {
    pub id: Option<String>,
    pub name: String,
    pub period: String,
    pub role: String,
    pub tech: String,
    pub summary: String,
}

/// YAMLで手書きするドキュメントのノード表現 (従来)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlNode {
    Simple(String),
    Structured(Box<YamlNodeStructured>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlNodeStructured {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub node_type: String, // "section", "para", "list", "table", "code", "term", "value", "math"
    
    // section の場合
    pub heading: Option<String>,
    
    // para の場合
    pub inline: Option<String>,
    
    // list の場合
    pub ordered: Option<bool>,
    
    // code / math の場合
    pub lang: Option<String>,
    pub src: Option<String>,
    
    // term の場合
    pub name: Option<String>,

    // value の場合
    pub scalar: Option<YamlScalar>,
    pub unit: Option<String>,
    
    // table の場合
    pub rows: Option<Vec<YamlDim>>,
    pub cols: Option<Vec<YamlDim>>,
    pub cells: Option<Vec<YamlCell>>,
    
    // contains 子要素
    pub children: Option<Vec<YamlNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum YamlScalar {
    Number(f64),
    Text(String),
    Bool(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDim {
    pub name: String,
    pub members: Vec<YamlMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlMember {
    pub key: String,
    pub label: Option<String>,
    #[serde(default)]
    pub children: Vec<YamlDim>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlCell {
    pub row: Vec<String>,
    pub col: Vec<String>,
    pub value: YamlCellValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case")]
pub enum YamlCellValue {
    Number { v: f64 },
    Text { v: String },
    Ref { to: String },
    Empty,
}

/// 解析したドキュメント
pub struct Vault {
    pub graph: Graph,
    pub root_id: NodeId,
}

pub struct Parser {
    key_to_id: HashMap<String, NodeId>,
    term_name_to_id: HashMap<String, NodeId>,
    term_nodes_to_add: Vec<Node>,
    edges_to_add: Vec<(NodeId, Rel, NodeId, Option<u32>)>,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            key_to_id: HashMap::new(),
            term_name_to_id: HashMap::new(),
            term_nodes_to_add: Vec::new(),
            edges_to_add: Vec::new(),
        }
    }

    pub fn get_or_create_id(&mut self, key: &str) -> NodeId {
        if let Some(id) = self.key_to_id.get(key) {
            *id
        } else {
            let id = NodeId::new();
            self.key_to_id.insert(key.to_string(), id);
            id
        }
    }

    pub fn get_or_create_term_id(&mut self, term_name: &str) -> NodeId {
        if let Some(id) = self.term_name_to_id.get(term_name) {
            *id
        } else {
            let id = NodeId::new();
            self.term_name_to_id.insert(term_name.to_string(), id);
            
            // Term ノード自体を後でグラフに追加する
            self.term_nodes_to_add.push(Node {
                id,
                payload: NodePayload::Term(Term {
                    name: term_name.to_string(),
                }),
            });
            id
        }
    }

    pub fn parse_vault(mut self, yaml_str: &str) -> Result<Vault, String> {
        let v: serde_yaml::Value = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("YAML parse error: {}", e))?;

        let mut graph = Graph::default();
        let root_id = if let Some(doc_type) = v.get("type").and_then(|t| t.as_str()) {
            match doc_type {
                "resume" => {
                    let resume: YamlResume = serde_yaml::from_value(v)
                        .map_err(|e| format!("YamlResume parse error: {}", e))?;
                    self.build_graph_from_resume(resume, &mut graph)?
                }
                "work_history" => {
                    let work_history: YamlWorkHistory = serde_yaml::from_value(v)
                        .map_err(|e| format!("YamlWorkHistory parse error: {}", e))?;
                    self.build_graph_from_work_history(work_history, &mut graph)?
                }
                _ => {
                    let root_node: YamlNode = serde_yaml::from_value(v)
                        .map_err(|e| format!("YamlNode parse error: {}", e))?;
                    self.parse_yaml_node(&root_node, &mut graph, None)?
                }
            }
        } else {
            let doc: YamlDocument = serde_yaml::from_value(v)
                .map_err(|e| format!("YAML parse error: {}", e))?;
            match doc {
                YamlDocument::Resume(resume) => {
                    self.build_graph_from_resume(*resume, &mut graph)?
                }
                YamlDocument::WorkHistory(work_history) => {
                    self.build_graph_from_work_history(*work_history, &mut graph)?
                }
                YamlDocument::Standard(root_node) => {
                    self.parse_yaml_node(&root_node, &mut graph, None)?
                }
            }
        };

        // 自動生成した Term ノードを追加
        for term_node in self.term_nodes_to_add {
            graph.insert(term_node);
        }

        // 収集したエッジを追加
        for (from, rel, to, ord) in self.edges_to_add {
            graph.link(from, rel, to, ord);
        }

        Ok(Vault { graph, root_id })
    }

    /// 人間用の YamlResume から canonical な Section / Table 等のグラフを構築する
    fn build_graph_from_resume(&mut self, r: YamlResume, graph: &mut Graph) -> Result<NodeId, String> {
        let root_id = NodeId::new();
        
        // 1. ルートセクション (履歴書)
        let root_node = Node {
            id: root_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("履歴書", root_id),
            }),
        };
        graph.insert(root_node);
        let mut ord = 0;

        // 2. プロフィール段落
        let profile_id = NodeId::new();
        let profile_text = format!(
            "氏名: {} ({})\n\n生年月日: {} (満{}歳)\n\n連絡先: {} / {}",
            r.name, r.name_yomi, r.birthday, r.age, r.contact.email, r.contact.tel
        );
        let profile_node = Node {
            id: profile_id,
            payload: NodePayload::Para(Para {
                inline: self.parse_inline_str(&profile_text, profile_id),
            }),
        };
        graph.insert(profile_node);
        graph.link(root_id, Rel::Contains, profile_id, Some(ord));
        ord += 1;

        // 3. 現住所セクション
        let address_sec_id = NodeId::new();
        let address_sec = Node {
            id: address_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("現住所", address_sec_id),
            }),
        };
        graph.insert(address_sec);
        graph.link(root_id, Rel::Contains, address_sec_id, Some(ord));
        ord += 1;

        let address_para_id = NodeId::new();
        let address_text = format!(
            "郵便番号: {}\n\n住所: {} ({})",
            r.contact.zip, r.contact.address, r.contact.address_yomi
        );
        let address_para = Node {
            id: address_para_id,
            payload: NodePayload::Para(Para {
                inline: self.parse_inline_str(&address_text, address_para_id),
            }),
        };
        graph.insert(address_para);
        graph.link(address_sec_id, Rel::Contains, address_para_id, Some(0));

        // 4. 学歴・職歴セクション
        let history_sec_id = NodeId::new();
        let history_sec = Node {
            id: history_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("学歴・職歴", history_sec_id),
            }),
        };
        graph.insert(history_sec);
        graph.link(root_id, Rel::Contains, history_sec_id, Some(ord));
        ord += 1;

        let table_id = NodeId::new();
        let mut row_members = Vec::new();
        let mut cells = Vec::new();

        if let Some(ref history_list) = r.history {
            for h in history_list {
                row_members.push(Member {
                    key: h.period.clone(),
                    label: None,
                    children: vec![],
                });
                cells.push(Cell {
                    row_path: vec![h.period.clone()],
                    col_path: vec!["org".to_string()],
                    value: CellValue::Text { v: h.org.clone() },
                });
                cells.push(Cell {
                    row_path: vec![h.period.clone()],
                    col_path: vec!["detail".to_string()],
                    value: CellValue::Text { v: h.detail.clone() },
                });
            }
        } else {
            let mut item_count = 0;
            if let Some(ref edu_list) = r.education {
                for e in edu_list {
                    let period = format!("{}年{}月", e.year, e.month);
                    let key = format!("edu_{}", item_count);
                    item_count += 1;
                    
                    row_members.push(Member {
                        key: key.clone(),
                        label: Some(self.parse_inline_str(&period, table_id)),
                        children: vec![],
                    });
                    cells.push(Cell {
                        row_path: vec![key.clone()],
                        col_path: vec!["org".to_string()],
                        value: CellValue::Text { v: "学歴".to_string() },
                    });
                    cells.push(Cell {
                        row_path: vec![key.clone()],
                        col_path: vec!["detail".to_string()],
                        value: CellValue::Text { v: e.description.clone() },
                    });
                }
            }
            if let Some(ref work_list) = r.work_history {
                for w in work_list {
                    let period = match (w.year, w.month) {
                        (Some(y), Some(m)) => format!("{}年{}月", y, m),
                        _ => "".to_string(),
                    };
                    let key = format!("work_{}", item_count);
                    item_count += 1;
                    
                    row_members.push(Member {
                        key: key.clone(),
                        label: Some(self.parse_inline_str(&period, table_id)),
                        children: vec![],
                    });
                    cells.push(Cell {
                        row_path: vec![key.clone()],
                        col_path: vec!["org".to_string()],
                        value: CellValue::Text { v: w.org.clone().unwrap_or_default() },
                    });
                    cells.push(Cell {
                        row_path: vec![key.clone()],
                        col_path: vec!["detail".to_string()],
                        value: CellValue::Text { v: w.detail.clone() },
                    });
                }
            }
        }

        let history_table = Table {
            rows: vec![Dim {
                name: "period".to_string(),
                members: row_members,
            }],
            cols: vec![Dim {
                name: "category".to_string(),
                members: vec![
                    Member { key: "org".to_string(), label: Some(self.parse_inline_str("組織名・学校名", table_id)), children: vec![] },
                    Member { key: "detail".to_string(), label: Some(self.parse_inline_str("詳細・専攻・役職", table_id)), children: vec![] },
                ],
            }],
            cells,
        };
        
        let table_node = Node {
            id: table_id,
            payload: NodePayload::Table(history_table),
        };
        graph.insert(table_node);
        graph.link(history_sec_id, Rel::Contains, table_id, Some(0));

        // 5. 免許・資格セクション
        let cert_sec_id = NodeId::new();
        let cert_sec = Node {
            id: cert_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("免許・資格", cert_sec_id),
            }),
        };
        graph.insert(cert_sec);
        graph.link(root_id, Rel::Contains, cert_sec_id, Some(ord));
        ord += 1;

        let cert_table_id = NodeId::new();
        let mut cert_row_members = Vec::new();
        let mut cert_cells = Vec::new();

        if let Some(ref cert_list) = r.certifications {
            for c in cert_list {
                cert_row_members.push(Member {
                    key: c.date.clone(),
                    label: None,
                    children: vec![],
                });
                cert_cells.push(Cell {
                    row_path: vec![c.date.clone()],
                    col_path: vec!["name".to_string()],
                    value: CellValue::Text { v: c.name.clone() },
                });
            }
        } else if let Some(ref license_list) = r.licenses {
            let mut lic_count = 0;
            for l in license_list {
                let date = format!("{}年{}月", l.year, l.month);
                let key = format!("lic_{}", lic_count);
                lic_count += 1;
                
                cert_row_members.push(Member {
                    key: key.clone(),
                    label: Some(self.parse_inline_str(&date, cert_table_id)),
                    children: vec![],
                });
                cert_cells.push(Cell {
                    row_path: vec![key.clone()],
                    col_path: vec!["name".to_string()],
                    value: CellValue::Text { v: l.name.clone() },
                });
            }
        }

        let cert_table = Table {
            rows: vec![Dim {
                name: "period".to_string(),
                members: cert_row_members,
            }],
            cols: vec![Dim {
                name: "category".to_string(),
                members: vec![
                    Member { key: "name".to_string(), label: Some(self.parse_inline_str("資格名称", cert_table_id)), children: vec![] },
                ],
            }],
            cells: cert_cells,
        };

        let cert_table_node = Node {
            id: cert_table_id,
            payload: NodePayload::Table(cert_table),
        };
        graph.insert(cert_table_node);
        graph.link(cert_sec_id, Rel::Contains, cert_table_id, Some(0));

        // 6. 自己PRセクション
        let motivation_sec_id = NodeId::new();
        let motivation_sec = Node {
            id: motivation_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("自己PR・志望動機", motivation_sec_id),
            }),
        };
        graph.insert(motivation_sec);
        graph.link(root_id, Rel::Contains, motivation_sec_id, Some(ord));

        let mut motivation_text = String::new();
        if let Some(ref mot) = r.motivation {
            motivation_text = mot.clone();
        } else {
            if let Some(ref h) = r.health {
                motivation_text.push_str(&format!("健康状態: {}\n\n", h));
            }
            if let Some(ref hob) = r.hobbies {
                motivation_text.push_str(&format!("趣味: {}\n\n", hob));
            }
            if let Some(ref sp) = r.sports {
                motivation_text.push_str(&format!("スポーツ: {}\n\n", sp));
            }
            if let Some(ref s) = r.spouse {
                motivation_text.push_str(&format!("配偶者: {}\n", if s.exists { "有" } else { "無" }));
            }
        }

        let motivation_para_id = NodeId::new();
        let motivation_para = Node {
            id: motivation_para_id,
            payload: NodePayload::Para(Para {
                inline: self.parse_inline_str(&motivation_text, motivation_para_id),
            }),
        };
        graph.insert(motivation_para);
        graph.link(motivation_sec_id, Rel::Contains, motivation_para_id, Some(0));

        Ok(root_id)
    }

    /// 人間用の YamlWorkHistory から canonical な Section / Table 等のグラフを構築する
    fn build_graph_from_work_history(&mut self, w: YamlWorkHistory, graph: &mut Graph) -> Result<NodeId, String> {
        let root_id = NodeId::new();

        // 1. ルートセクション (職務経歴書)
        let root_node = Node {
            id: root_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("職務経歴書", root_id),
            }),
        };
        graph.insert(root_node);
        let mut ord = 0;

        // 2. プロフィール段落
        let profile_id = NodeId::new();
        let profile_text = format!("氏名: {}\n\n職種: {}", w.name, w.title);
        let profile_node = Node {
            id: profile_id,
            payload: NodePayload::Para(Para {
                inline: self.parse_inline_str(&profile_text, profile_id),
            }),
        };
        graph.insert(profile_node);
        graph.link(root_id, Rel::Contains, profile_id, Some(ord));
        ord += 1;

        // 3. 職務要約セクション
        let summary_sec_id = NodeId::new();
        let summary_sec = Node {
            id: summary_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("職務要約", summary_sec_id),
            }),
        };
        graph.insert(summary_sec);
        graph.link(root_id, Rel::Contains, summary_sec_id, Some(ord));
        ord += 1;

        let summary_para_id = NodeId::new();
        let summary_para = Node {
            id: summary_para_id,
            payload: NodePayload::Para(Para {
                inline: self.parse_inline_str(&w.summary, summary_para_id),
            }),
        };
        graph.insert(summary_para);
        graph.link(summary_sec_id, Rel::Contains, summary_para_id, Some(0));

        // 4. 技術スタックセクション
        let tech_sec_id = NodeId::new();
        let tech_sec = Node {
            id: tech_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("技術スタック", tech_sec_id),
            }),
        };
        graph.insert(tech_sec);
        graph.link(root_id, Rel::Contains, tech_sec_id, Some(ord));
        ord += 1;

        let tech_table_id = NodeId::new();
        let mut tech_rows = Vec::new();
        let mut tech_cells = Vec::new();

        for t in &w.tech_stack {
            tech_rows.push(Member {
                key: t.category.clone(),
                label: Some(self.parse_inline_str(&t.label, tech_table_id)),
                children: vec![],
            });
            tech_cells.push(Cell {
                row_path: vec![t.category.clone()],
                col_path: vec!["details".to_string()],
                value: CellValue::Text { v: t.tech.clone() },
            });
            tech_cells.push(Cell {
                row_path: vec![t.category.clone()],
                col_path: vec!["level".to_string()],
                value: CellValue::Text { v: t.level.clone() },
            });
        }

        let tech_table = Table {
            rows: vec![Dim {
                name: "category".to_string(),
                members: tech_rows,
            }],
            cols: vec![Dim {
                name: "tech_details".to_string(),
                members: vec![
                    Member { key: "details".to_string(), label: Some(self.parse_inline_str("技術要素", tech_table_id)), children: vec![] },
                    Member { key: "level".to_string(), label: Some(self.parse_inline_str("経験・スキル感", tech_table_id)), children: vec![] },
                ],
            }],
            cells: tech_cells,
        };

        let tech_table_node = Node {
            id: tech_table_id,
            payload: NodePayload::Table(tech_table),
        };
        graph.insert(tech_table_node);
        graph.link(tech_sec_id, Rel::Contains, tech_table_id, Some(0));

        // 5. 職務詳細セクション
        let proj_sec_id = NodeId::new();
        let proj_sec = Node {
            id: proj_sec_id,
            payload: NodePayload::Section(Section {
                heading: self.parse_inline_str("職務詳細", proj_sec_id),
            }),
        };
        graph.insert(proj_sec);
        graph.link(root_id, Rel::Contains, proj_sec_id, Some(ord));

        let proj_table_id = NodeId::new();
        let mut proj_rows = Vec::new();
        let mut proj_cells = Vec::new();

        for (i, p) in w.projects.iter().enumerate() {
            let key = p.id.clone().unwrap_or_else(|| format!("project_{}", i));
            proj_rows.push(Member {
                key: key.clone(),
                label: Some(self.parse_inline_str(&p.name, proj_table_id)),
                children: vec![],
            });
            proj_cells.push(Cell {
                row_path: vec![key.clone()],
                col_path: vec!["period".to_string()],
                value: CellValue::Text { v: p.period.clone() },
            });
            proj_cells.push(Cell {
                row_path: vec![key.clone()],
                col_path: vec!["role".to_string()],
                value: CellValue::Text { v: p.role.clone() },
            });
            proj_cells.push(Cell {
                row_path: vec![key.clone()],
                col_path: vec!["tech".to_string()],
                value: CellValue::Text { v: p.tech.clone() },
            });
            proj_cells.push(Cell {
                row_path: vec![key.clone()],
                col_path: vec!["summary".to_string()],
                value: CellValue::Text { v: p.summary.clone() },
            });
        }

        let proj_table = Table {
            rows: vec![Dim {
                name: "project".to_string(),
                members: proj_rows,
            }],
            cols: vec![Dim {
                name: "project_attrs".to_string(),
                members: vec![
                    Member { key: "period".to_string(), label: Some(self.parse_inline_str("期間", proj_table_id)), children: vec![] },
                    Member { key: "role".to_string(), label: Some(self.parse_inline_str("役割", proj_table_id)), children: vec![] },
                    Member { key: "tech".to_string(), label: Some(self.parse_inline_str("主要技術", proj_table_id)), children: vec![] },
                    Member { key: "summary".to_string(), label: Some(self.parse_inline_str("業務内容・成果", proj_table_id)), children: vec![] },
                ],
            }],
            cells: proj_cells,
        };

        let proj_table_node = Node {
            id: proj_table_id,
            payload: NodePayload::Table(proj_table),
        };
        graph.insert(proj_table_node);
        graph.link(proj_sec_id, Rel::Contains, proj_table_id, Some(0));

        Ok(root_id)
    }

    fn parse_yaml_node(
        &mut self,
        yaml: &YamlNode,
        graph: &mut Graph,
        parent_info: Option<(NodeId, u32)>, // (親ID, インデックス)
    ) -> Result<NodeId, String> {
        match yaml {
            YamlNode::Simple(s) => {
                let node_id = NodeId::new();
                let inline = self.parse_inline_str(s, node_id);
                
                let node = Node {
                    id: node_id,
                    payload: NodePayload::Para(Para { inline }),
                };
                graph.insert(node);

                if let Some((parent_id, ord)) = parent_info {
                    graph.link(parent_id, Rel::Contains, node_id, Some(ord));
                }

                Ok(node_id)
            }
            YamlNode::Structured(box_structured) => {
                let s = box_structured.as_ref();
                
                let node_id = if let Some(ref key) = s.id {
                    self.get_or_create_id(key)
                } else {
                    NodeId::new()
                };

                let payload = match s.node_type.as_str() {
                    "section" => {
                        let heading_str = s.heading.as_deref().unwrap_or("");
                        let heading = self.parse_inline_str(heading_str, node_id);
                        NodePayload::Section(Section { heading })
                    }
                    "para" => {
                        let inline_str = s.inline.as_deref().unwrap_or("");
                        let inline = self.parse_inline_str(inline_str, node_id);
                        NodePayload::Para(Para { inline })
                    }
                    "list" => {
                        NodePayload::List(List {
                            ordered: s.ordered.unwrap_or(false),
                        })
                    }
                    "code" => {
                        NodePayload::Code(Code {
                            lang: s.lang.clone().unwrap_or_default(),
                            src: s.src.clone().unwrap_or_default(),
                        })
                    }
                    "term" => {
                        let term_name = s.name.clone().unwrap_or_default();
                        NodePayload::Term(Term { name: term_name })
                    }
                    "value" => {
                        let scalar = match s.scalar {
                            Some(YamlScalar::Number(v)) => Scalar::Number(v),
                            Some(YamlScalar::Text(ref v)) => Scalar::Text(v.clone()),
                            Some(YamlScalar::Bool(v)) => Scalar::Bool(v),
                            None => Scalar::Text(String::new()),
                        };
                        NodePayload::Value(Value {
                            scalar,
                            unit: s.unit.clone(),
                        })
                    }
                    "math" => {
                        let src_str = s.src.as_deref().unwrap_or("");
                        let tree = match tex2math::parse_normalized(src_str) {
                            Ok(t) => {
                                let json = serde_json::to_string(&t).unwrap();
                                serde_json::from_str(&json).unwrap()
                            }
                            Err(e) => {
                                return Err(format!("Math parse error inside node: {:?}", e));
                            }
                        };
                        NodePayload::Math(MathBlock { tree })
                    }
                    "table" => {
                        let rows = s.rows.as_ref().map(|r| self.convert_dims(r, node_id)).unwrap_or_default();
                        let cols = s.cols.as_ref().map(|c| self.convert_dims(c, node_id)).unwrap_or_default();
                        let cells = s.cells.as_ref().map(|c| self.convert_cells(c, node_id)).unwrap_or_default();
                        NodePayload::Table(Table { rows, cols, cells })
                    }
                    unknown => {
                        return Err(format!("Unknown node type: {}", unknown));
                    }
                };

                let node = Node { id: node_id, payload };
                graph.insert(node);

                if let Some((parent_id, ord)) = parent_info {
                    graph.link(parent_id, Rel::Contains, node_id, Some(ord));
                }

                if let Some(ref children) = s.children {
                    for (i, child) in children.iter().enumerate() {
                        self.parse_yaml_node(child, graph, Some((node_id, i as u32)))?;
                    }
                }

                Ok(node_id)
            }
        }
    }

    fn convert_dims(&mut self, yaml_dims: &[YamlDim], node_id: NodeId) -> DimTree {
        yaml_dims.iter().map(|d| {
            Dim {
                name: d.name.clone(),
                members: d.members.iter().map(|m| {
                    let label = m.label.as_ref().map(|l| self.parse_inline_str(l, node_id));
                    Member {
                        key: m.key.clone(),
                        label,
                        children: self.convert_dims(&m.children, node_id),
                    }
                }).collect(),
            }
        }).collect()
    }

    fn convert_cells(&mut self, yaml_cells: &[YamlCell], node_id: NodeId) -> Vec<Cell> {
        yaml_cells.iter().map(|c| {
            let value = match &c.value {
                YamlCellValue::Number { v } => CellValue::Number { v: *v },
                YamlCellValue::Text { v } => CellValue::Text { v: v.clone() },
                YamlCellValue::Ref { to } => {
                    let target_id = self.get_or_create_id(to);
                    self.edges_to_add.push((node_id, Rel::DependsOn, target_id, None));
                    CellValue::Ref { to: target_id }
                }
                YamlCellValue::Empty => CellValue::Empty,
            };
            Cell {
                row_path: c.row.clone(),
                col_path: c.col.clone(),
                value,
            }
        }).collect()
    }

    fn parse_inline_str(&mut self, s: &str, current_node_id: NodeId) -> Vec<Inline> {
        let mut result = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i+1] == '*' {
                i += 2;
                let mut content = String::new();
                let mut closed = false;
                while i < chars.len() {
                    if i + 1 < chars.len() && chars[i] == '*' && chars[i+1] == '*' {
                        i += 2;
                        closed = true;
                        break;
                    }
                    content.push(chars[i]);
                    i += 1;
                }
                let children = if closed {
                    self.parse_inline_str(&content, current_node_id)
                } else {
                    vec![Inline::Text { s: format!("**{}", content) }]
                };
                result.push(Inline::Emph {
                    kind: EmphKind::Strong,
                    children,
                });
                continue;
            }
            
            if chars[i] == '*' {
                i += 1;
                let mut content = String::new();
                let mut closed = false;
                while i < chars.len() {
                    if chars[i] == '*' {
                        i += 1;
                        closed = true;
                        break;
                    }
                    content.push(chars[i]);
                    i += 1;
                }
                let children = if closed {
                    self.parse_inline_str(&content, current_node_id)
                } else {
                    vec![Inline::Text { s: format!("*{}", content) }]
                };
                result.push(Inline::Emph {
                    kind: EmphKind::Em,
                    children,
                });
                continue;
            }
            
            if chars[i] == '`' {
                i += 1;
                let mut content = String::new();
                let mut closed = false;
                while i < chars.len() {
                    if chars[i] == '`' {
                        i += 1;
                        closed = true;
                        break;
                    }
                    content.push(chars[i]);
                    i += 1;
                }
                if closed {
                    result.push(Inline::Emph {
                        kind: EmphKind::Code,
                        children: vec![Inline::Text { s: content }],
                    });
                } else {
                    result.push(Inline::Text { s: format!("`{}", content) });
                }
                continue;
            }
            
            if chars[i] == '$' {
                i += 1;
                let mut content = String::new();
                let mut closed = false;
                while i < chars.len() {
                    if chars[i] == '$' {
                        i += 1;
                        closed = true;
                        break;
                    }
                    content.push(chars[i]);
                    i += 1;
                }
                if closed {
                    match tex2math::parse_normalized(&content) {
                        Ok(tree) => {
                            let json = serde_json::to_string(&tree).unwrap();
                            let core_tree: strata_core::MathNode = serde_json::from_str(&json).unwrap();
                            result.push(Inline::Math { tree: core_tree });
                        }
                        Err(_) => {
                            result.push(Inline::Text { s: format!("${}$", content) });
                        }
                    }
                } else {
                    result.push(Inline::Text { s: format!("${}", content) });
                }
                continue;
            }
            
            if chars[i] == '[' {
                i += 1;
                let mut label = String::new();
                let mut label_closed = false;
                while i < chars.len() {
                    if chars[i] == ']' {
                        i += 1;
                        label_closed = true;
                        break;
                    }
                    label.push(chars[i]);
                    i += 1;
                }
                
                if label_closed && i < chars.len() && chars[i] == '(' {
                    i += 1;
                    let mut dest = String::new();
                    let mut dest_closed = false;
                    while i < chars.len() {
                        if chars[i] == ')' {
                            i += 1;
                            dest_closed = true;
                            break;
                        }
                        dest.push(chars[i]);
                        i += 1;
                    }
                    
                    if dest_closed {
                        if dest.starts_with("ref:") {
                            let key = dest["ref:".len()..].to_string();
                            let target_id = self.get_or_create_id(&key);
                            result.push(Inline::Ref { to: target_id, rel: Rel::DependsOn });
                            self.edges_to_add.push((current_node_id, Rel::DependsOn, target_id, None));
                        } else if dest.starts_with("term:") {
                            let term_name = dest["term:".len()..].to_string();
                            let term_id = self.get_or_create_term_id(&term_name);
                            result.push(Inline::Term { to: term_id });
                            self.edges_to_add.push((current_node_id, Rel::TermRef, term_id, None));
                        } else {
                            result.push(Inline::Text { s: format!("[{}](invalid-link:{})", label, dest) });
                        }
                        continue;
                    } else {
                        result.push(Inline::Text { s: format!("[{}({}", label, dest) });
                        continue;
                    }
                } else {
                    result.push(Inline::Text { s: format!("[{}", label) });
                    continue;
                }
            }
            
            let mut text = String::new();
            while i < chars.len() {
                let c = chars[i];
                if c == '*' || c == '`' || c == '$' || c == '[' {
                    break;
                }
                text.push(c);
                i += 1;
            }
            result.push(Inline::Text { s: text });
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_yaml() {
        let yaml = r#"
type: section
heading: "テストセクション"
id: root_sec
children:
  - type: para
    inline: "これは [Rust](term:Rust) の紹介です。**太字** や $x^2$ があります。"
  - type: para
    inline: "ルートのIDは [ここ](ref:root_sec) です。"
"#;
        let parser = Parser::new();
        let vault = parser.parse_vault(yaml).unwrap();
        assert_eq!(vault.graph.nodes.len(), 4); // RootSection, Para1, Para2, Term(Rust)
        assert!(strata_core::invariants::validate(&vault.graph).is_empty());
    }
}
