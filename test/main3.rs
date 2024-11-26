use csv::Writer;
use reqwest::header::USER_AGENT;
use std::{ error::Error, fs::File, io::{ self, Write }, path::PathBuf };
use select::{ document::Document, predicate::{ Attr, Name, Predicate } };

fn create_folder_if_not_exists(folder_path: &str) -> Result<PathBuf, std::io::Error> {
    let folder_path = PathBuf::from(folder_path);

    if folder_path.exists() {
        Ok(folder_path)
    } else {
        std::fs::create_dir_all(&folder_path)?;
        Ok(folder_path)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let folder_path = "bgp.tools";
    match create_folder_if_not_exists(folder_path) {
        Ok(_path) => {}
        Err(e) => eprintln!("Error creating folder: {}", e),
    }
    let asn = get_user_input();

    let url = format!("https://bgp.tools/as/{asn}#prefixes");

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
        )
        .send().await?;

    if response.status().is_success() {
        let mut writer = Writer::from_path(format!("{folder_path}/AS{asn}.csv")).unwrap();
        writer.write_record(vec!["IP地址前缀", "国家代码", "描述"]).unwrap();

        let mut file = File::create(format!("{folder_path}/AS{asn}.txt")).expect(
            "Failed to create file"
        );

        // 获取 HTML 内容为字符串
        let content = response.text().await?;

        // 使用 select 解析 HTML
        let document = Document::from(content.as_str());

        println!();

        // 找到表格的所有行
        for row in document.find(
            Attr("id", "donotscrapebgptools-prefixlist-tbody").descendant(Name("tr"))
        ) {
            let cells: Vec<_> = row
                .find(Name("td"))
                .map(|cell: select::node::Node<'_>| {
                    // 创建一个向量，用于存储结果
                    let mut elements = Vec::new();

                    // 查找 img 元素的国家代码
                    if let Some(img) = cell.find(Name("img")).next() {
                        // 获取第一个 img
                        if let Some(title) = img.attr("title") {
                            // 获取第一个 img 的 title
                            elements.push(title.to_string());
                        }
                    }

                    // 添加 td 的文本内容
                    let text = cell.text().trim().to_string();
                    if !text.is_empty() {
                        elements.push(text);
                    } else {
                        elements.push("".to_string()); // 添加空字符串，占位
                    }

                    elements // 返回当前单元格解析出的内容
                })
                .collect();
            if !cells.is_empty() {
                let one_dimensional: Vec<String> = cells.into_iter().flatten().collect(); // 将二维向量转换为一维向量
                // 调整元素排列顺序，以及过滤掉不要的元素
                let transformed_vec = vec![
                    one_dimensional
                        .get(2)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()), // 第3个元素
                    one_dimensional
                        .get(0)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()), // 第1个元素
                    one_dimensional
                        .get(3)
                        .cloned()
                        .unwrap_or_else(|| "".to_string()) // 第4个元素
                ];
                println!("抓取到内容：{:?}", transformed_vec);
                writer.write_record(&transformed_vec).unwrap();
                writeln!(file, "{}", one_dimensional[2]).unwrap();
            }
        }
    } else {
        println!("Failed to fetch the page. Status: {}", response.status());
    }

    Ok(())
}

fn get_user_input() -> String {
    loop {
        print!("请输入要下载的AS(可以带有'AS'前缀):");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_uppercase();

        if let Some(num) = input.strip_prefix("AS").and_then(|s| s.parse::<usize>().ok()) {
            if (1..=999999).contains(&num) {
                return format!("{}", num);
            }
        } else if let Ok(num) = input.parse::<usize>() {
            if (1..=999999).contains(&num) {
                return format!("{}", num);
            }
        }
    }
}
