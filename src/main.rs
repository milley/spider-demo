use std::cmp::min;
use std::fs::File;
use std::io::Write;

use scraper::{Html, Selector};
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;

pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), String> {
    // Reqwest setup
    let res = client
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;
    
    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", url));

    // download chunks
    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write_all(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, path));
    return Ok(());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let body = reqwest::get("https://mp.weixin.qq.com/s?__biz=MzI3MTYxMDI0OA==&mid=2247516625&idx=3&sn=4a59a7bd04f3f4048402589583b3231b&chksm=eb3df936dc4a70209ac505168f301a9de78d7642e70554f7903386875bfbbcb8f28d63225e7d&scene=21#wechat_redirect")
    .await?
    .text()
    .await?;

    let client = reqwest::Client::new();
    let mut module_url_vec: Vec<String> = vec![];
    // println!("{:#?}", body);

    let document = scraper::Html::parse_document(&body);

    // 1. analyzer module url
    let main_div_selector = scraper::Selector::parse("div.rich_media_wrp").unwrap();
    let main_div = document.select(&main_div_selector).next().unwrap();

    let u_div_selector = scraper::Selector::parse("div.rich_media_content>p>a").unwrap();
    
    for div in main_div.select(&u_div_selector) {
        let module_url = div.value().attr("href").unwrap();
        // println!("{}", module_url);
        module_url_vec.push(module_url.to_string());
    }

    // 2. open each module url, analyzer each mp3 url
    for module_url in module_url_vec {
        let module_body = reqwest::get(module_url)
        .await?
        .text()
        .await?;

        // println!("{}", module_body);

        let module_document = scraper::Html::parse_document(&module_body);

        // find each mp3 url
        let module_div_selector = scraper::Selector::parse("div.rich_media_content").unwrap();
        let module_div = module_document.select(&module_div_selector).next().unwrap();


        let module_mpvoice_selector = scraper::Selector::parse("section").unwrap();
        for item in module_div.select(&module_mpvoice_selector) {
            // println!("{}", item.inner_html());

            let mpvoice_html = item.inner_html();

            if mpvoice_html.find("mpvoice") != None {
                let fragment = Html::parse_fragment(&mpvoice_html);
                let mpvoice_selector = Selector::parse("mpvoice").unwrap();
                let mp_voice = fragment.select(&mpvoice_selector).next().unwrap();

                let mpvoice_name = mp_voice.value().attr("name").unwrap().replace("\u{a0}", "_") + ".mp3";
                let mpvoice_filepath = "./download/".to_owned() + mpvoice_name.as_str();
                let mpvoice_url = "https://res.wx.qq.com/voice/getvoice?mediaid=".to_owned() + mp_voice.value().attr("voice_encode_fileid").unwrap();
                println!("{:?}-{:?}", mpvoice_filepath, mpvoice_url);

                download_file(&client, mpvoice_url.as_str(), mpvoice_filepath.as_str()).await.unwrap();
            }
        }
    }

	
	Ok(())
}