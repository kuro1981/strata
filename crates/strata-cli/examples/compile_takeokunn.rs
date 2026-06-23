use strata_core::{
    Graph, Node, NodeId, NodePayload, Para, Rel, Section, Table, Term, Value, Scalar, CellValue, DimTree, Inline
};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
struct Profile {
    #[serde(rename = "性読み")]
    sei_yomi: String,
    #[serde(rename = "名読み")]
    mei_yomi: String,
    #[serde(rename = "性")]
    sei: String,
    #[serde(rename = "名")]
    mei: String,
    #[serde(rename = "生年月日")]
    birthday: String,
    #[serde(rename = "満年齢")]
    age: String,
    #[serde(rename = "写真")]
    photo: String,
}

#[derive(Serialize)]
struct Address {
    #[serde(rename = "住所ふりがな1")]
    address_yomi_1: String,
    #[serde(rename = "住所1")]
    address_1: String,
    #[serde(rename = "郵便番号1")]
    zip_1: String,
    #[serde(rename = "電話番号1")]
    tel_1: String,
    #[serde(rename = "Email1")]
    email_1: String,
    #[serde(rename = "住所ふりがな2")]
    address_yomi_2: String,
    #[serde(rename = "住所2")]
    address_2: String,
    #[serde(rename = "郵便番号2")]
    zip_2: String,
    #[serde(rename = "電話番号2")]
    tel_2: String,
    #[serde(rename = "Email2")]
    email_2: String,
}

#[derive(Serialize)]
struct EducationEntry {
    #[serde(rename = "年")]
    year: String,
    #[serde(rename = "月")]
    month: String,
    #[serde(rename = "学歴")]
    detail: String,
}

#[derive(Serialize)]
struct WorkEntry {
    #[serde(rename = "年")]
    year: String,
    #[serde(rename = "月")]
    month: String,
    #[serde(rename = "職歴")]
    detail: String,
}

#[derive(Serialize)]
struct CertificationEntry {
    #[serde(rename = "年")]
    year: String,
    #[serde(rename = "月")]
    month: String,
    #[serde(rename = "資格")]
    detail: String,
}

#[derive(Serialize)]
struct MotivationRequests {
    #[serde(rename = "志望動機")]
    motivation: String,
    #[serde(rename = "本人希望")]
    requests: String,
}

fn main() {
    let input_path = Path::new("vault/resume.yaml");
    println!("Loading Strata document from {:?}", input_path);
    
    let yaml_str = fs::read_to_string(input_path).expect("Failed to read input yaml");
    let parser = strata_vault::Parser::new();
    let vault = parser.parse_vault(&yaml_str).expect("Failed to parse Strata document");
    
    let graph = &vault.graph;
    
    // データ抽出用変数
    let mut profile = Profile {
        sei_yomi: "".into(), mei_yomi: "".into(),
        sei: "".into(), mei: "".into(),
        birthday: "".into(), age: "".into(), photo: "".into(),
    };
    
    let mut address = Address {
        address_yomi_1: "".into(), address_1: "".into(), zip_1: "".into(),
        tel_1: "".into(), email_1: "".into(),
        address_yomi_2: "".into(), address_2: "".into(), zip_2: "".into(),
        tel_2: "".into(), email_2: "".into(),
    };
    
    let mut education_history = Vec::new();
    let mut work_history = Vec::new();
    let mut certifications = Vec::new();
    let mut motivation_requests = MotivationRequests {
        motivation: "".into(),
        requests: "".into(),
    };
    
    // グラフの走査
    let children = graph.children_of(vault.root_id);
    let mut child_index = 0;
    
    for child_id in children {
        let node = graph.nodes.get(&child_id).unwrap();
        match &node.payload {
            NodePayload::Para(p) => {
                if child_index == 0 {
                    // プロフィール情報
                    let text = inlines_to_string(&p.inline);
                    
                    // 氏名抽出
                    if let Some(name_section) = extract_field(&text, "氏名:", "\n") {
                        // "黒田 太郎 (くろだ たろう)"
                        let parts: Vec<&str> = name_section.split('(').collect();
                        if parts.len() >= 1 {
                            let kanji = parts[0].trim();
                            let kanji_parts: Vec<&str> = kanji.split_whitespace().collect();
                            if kanji_parts.len() >= 2 {
                                profile.sei = kanji_parts[0].to_string();
                                profile.mei = kanji_parts[1].to_string();
                            } else {
                                profile.sei = kanji.to_string();
                            }
                        }
                        if parts.len() >= 2 {
                            let yomi = parts[1].replace(')', "");
                            let yomi = yomi.trim();
                            let yomi_parts: Vec<&str> = yomi.split_whitespace().collect();
                            if yomi_parts.len() >= 2 {
                                profile.sei_yomi = yomi_parts[0].to_string();
                                profile.mei_yomi = yomi_parts[1].to_string();
                            } else {
                                profile.sei_yomi = yomi.to_string();
                            }
                        }
                    }
                    
                    // 生年月日・年齢
                    if let Some(bday) = extract_field(&text, "生年月日:", "\n") {
                        // "1990年1月1日 (満36歳)"
                        let parts: Vec<&str> = bday.split('(').collect();
                        if parts.len() >= 1 {
                            profile.birthday = parts[0].trim().to_string();
                        }
                        if parts.len() >= 2 {
                            profile.age = parts[1].replace("満", "").replace("歳", "").replace(')', "").trim().to_string();
                        }
                    }
                    
                    // 連絡先
                    if let Some(contact) = extract_field(&text, "連絡先:", "") {
                        // "kuro@example.com / 090-XXXX-XXXX"
                        let parts: Vec<&str> = contact.split('/').collect();
                        if parts.len() >= 1 {
                            address.email_1 = parts[0].trim().to_string();
                        }
                        if parts.len() >= 2 {
                            address.tel_1 = parts[1].trim().to_string();
                        }
                    }
                }
                child_index += 1;
            }
            NodePayload::Section(s) => {
                let section_title = inlines_to_string(&s.heading);
                if section_title == "現住所" {
                    // 現住所セクションの子Paraを解析
                    let sec_children = graph.children_of(child_id);
                    if let Some(para_id) = sec_children.first() {
                        if let Some(para_node) = graph.nodes.get(para_id) {
                            if let NodePayload::Para(p) = &para_node.payload {
                                let text = inlines_to_string(&p.inline);
                                address.zip_1 = extract_field(&text, "郵便番号:", "\n").unwrap_or_default();
                                
                                if let Some(addr_sec) = extract_field(&text, "住所:", "") {
                                    // "東京都豊島区豊島 (とうきょうと としまく としま)"
                                    let parts: Vec<&str> = addr_sec.split('(').collect();
                                    if parts.len() >= 1 {
                                        address.address_1 = parts[0].trim().to_string();
                                    }
                                    if parts.len() >= 2 {
                                        address.address_yomi_1 = parts[1].replace(')', "").trim().to_string();
                                    }
                                }
                            }
                        }
                    }
                } else if section_title == "学歴・職歴" {
                    // 学歴・職歴テーブルの解析
                    let sec_children = graph.children_of(child_id);
                    if let Some(table_id) = sec_children.first() {
                        if let Some(table_node) = graph.nodes.get(table_id) {
                            if let NodePayload::Table(t) = &table_node.payload {
                                parse_history_table(t, &mut education_history, &mut work_history);
                            }
                        }
                    }
                } else if section_title == "免許・資格" {
                    // 免許・資格テーブルの解析
                    let sec_children = graph.children_of(child_id);
                    if let Some(table_id) = sec_children.first() {
                        if let Some(table_node) = graph.nodes.get(table_id) {
                            if let NodePayload::Table(t) = &table_node.payload {
                                parse_cert_table(t, &mut certifications);
                            }
                        }
                    }
                } else if section_title == "自己PR・志望動機" {
                    // 自己PR・志望動機
                    let sec_children = graph.children_of(child_id);
                    if let Some(para_id) = sec_children.first() {
                        if let Some(para_node) = graph.nodes.get(para_id) {
                            if let NodePayload::Para(p) = &para_node.payload {
                                motivation_requests.motivation = inlines_to_string(&p.inline);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    // YAMLファイル群の保存先
    let target_dir = Path::new("/home/kuro/dev/typst-resume-template/resume/content");
    println!("Saving YAML files to {:?}", target_dir);
    fs::create_dir_all(target_dir).expect("Failed to create target content directory");
    
    write_yaml(target_dir.join("profile.yaml"), &profile);
    write_yaml(target_dir.join("address.yaml"), &address);
    write_yaml(target_dir.join("education_history.yaml"), &education_history);
    write_yaml(target_dir.join("work_history.yaml"), &work_history);
    write_yaml(target_dir.join("certifications.yaml"), &certifications);
    write_yaml(target_dir.join("motivation_requests.yaml"), &motivation_requests);
    
    println!("Done exporting data. Running typst compilation...");
    
    // Typst コンパイルの実行
    let typst_cwd = Path::new("/home/kuro/dev/typst-resume-template/resume");
    let status = Command::new("typst")
        .args(&["compile", "main.typ", "resume.pdf"])
        .current_dir(typst_cwd)
        .status();
        
    match status {
        Ok(s) if s.success() => {
            println!("Successfully compiled resume.pdf!");
            // PDFをプロジェクトルートへコピー
            fs::copy(
                typst_cwd.join("resume.pdf"),
                Path::new("resume_takeokunn.pdf")
            ).expect("Failed to copy PDF output to project root");
            println!("Copied compiled PDF to ./resume_takeokunn.pdf");
        }
        Ok(s) => {
            eprintln!("Typst exited with error code: {:?}", s.code());
        }
        Err(e) => {
            eprintln!("Failed to execute typst compile command: {}", e);
            eprintln!("Note: Please make sure 'typst' CLI is installed in your system path.");
        }
    }
}

fn inlines_to_string(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { s } => out.push_str(s),
            Inline::Emph { children, .. } => out.push_str(&inlines_to_string(children)),
            Inline::Math { .. } => out.push_str("[Math]"),
            Inline::Ref { .. } => out.push_str("[Ref]"),
            Inline::Term { .. } => out.push_str("[Term]"),
            Inline::Anchor { .. } => out.push_str("[Anchor]"),
        }
    }
    out
}

fn extract_field(s: &str, prefix: &str, suffix: &str) -> Option<String> {
    if let Some(pos) = s.find(prefix) {
        let start = pos + prefix.len();
        let rest = &s[start..];
        let end = if suffix.is_empty() {
            rest.len()
        } else {
            rest.find(suffix).unwrap_or(rest.len())
        };
        Some(rest[..end].trim().to_string())
    } else {
        None
    }
}

fn write_yaml<T: Serialize, P: AsRef<Path>>(path: P, val: &T) {
    let s = serde_yaml::to_string(val).expect("Failed to serialize yaml");
    fs::write(path, s).expect("Failed to write yaml file");
}

fn parse_history_table(
    table: &Table,
    edu: &mut Vec<EducationEntry>,
    work: &mut Vec<WorkEntry>,
) {
    // cells を (row, col) -> value のマップにする
    let mut cell_map = HashMap::new();
    for cell in &table.cells {
        if cell.row_path.len() >= 1 && cell.col_path.len() >= 1 {
            cell_map.insert((cell.row_path[0].clone(), cell.col_path[0].clone()), &cell.value);
        }
    }
    
    if table.rows.is_empty() { return; }
    
    // 行を走査
    for member in &table.rows[0].members {
        let period = &member.key; // "2008-04 ~ 2012-03"
        let org = match cell_map.get(&(period.clone(), "org".to_string())) {
            Some(CellValue::Text { v }) => v.clone(),
            _ => "".to_string(),
        };
        let detail = match cell_map.get(&(period.clone(), "detail".to_string())) {
            Some(CellValue::Text { v }) => v.clone(),
            _ => "".to_string(),
        };
        
        // 学歴か職歴かの判定
        let is_edu = org.contains("大学") || org.contains("高校") || org.contains("学校");
        
        // 期間の分解
        let parts: Vec<&str> = period.split('~').collect();
        if parts.len() >= 1 {
            let start = parts[0].trim(); // "2008-04"
            let start_parts: Vec<&str> = start.split('-').collect();
            if start_parts.len() >= 2 {
                let yr = start_parts[0].trim().to_string();
                let mo = start_parts[1].trim().trim_start_matches('0').to_string();
                
                if is_edu {
                    edu.push(EducationEntry {
                        year: yr, month: mo,
                        detail: format!("{} 入学", org),
                    });
                } else {
                    work.push(WorkEntry {
                        year: yr, month: mo,
                        detail: format!("{} 入社", org),
                    });
                }
            }
        }
        
        if parts.len() >= 2 {
            let end = parts[1].trim(); // "2012-03" または "現在"
            if end != "現在" {
                let end_parts: Vec<&str> = end.split('-').collect();
                if end_parts.len() >= 2 {
                    let yr = end_parts[0].trim().to_string();
                    let mo = end_parts[1].trim().trim_start_matches('0').to_string();
                    
                    if is_edu {
                        edu.push(EducationEntry {
                            year: yr, month: mo,
                            detail: format!("{} {}", org, detail),
                        });
                    } else {
                        let clean_detail = if detail.contains("退社") || detail.contains("退職") {
                            detail
                        } else {
                            "退社".to_string()
                        };
                        work.push(WorkEntry {
                            year: yr, month: mo,
                            detail: format!("{} {}", org, clean_detail),
                        });
                    }
                }
            }
        }
    }
}

fn parse_cert_table(table: &Table, certs: &mut Vec<CertificationEntry>) {
    let mut cell_map = HashMap::new();
    for cell in &table.cells {
        if cell.row_path.len() >= 1 && cell.col_path.len() >= 1 {
            cell_map.insert((cell.row_path[0].clone(), cell.col_path[0].clone()), &cell.value);
        }
    }
    
    if table.rows.is_empty() { return; }
    
    for member in &table.rows[0].members {
        let period = &member.key; // "2011-06"
        let name = match cell_map.get(&(period.clone(), "name".to_string())) {
            Some(CellValue::Text { v }) => v.clone(),
            _ => "".to_string(),
        };
        
        let parts: Vec<&str> = period.split('-').collect();
        if parts.len() >= 2 {
            let yr = parts[0].trim().to_string();
            let mo = parts[1].trim().trim_start_matches('0').to_string();
            certs.push(CertificationEntry {
                year: yr, month: mo,
                detail: name,
            });
        }
    }
}
