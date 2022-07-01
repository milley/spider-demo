extern crate spider;

use spider::website::Website;

fn main() {
    let mut website: Website = Website::new("https://choosealicense.com");
    website.configuration.blacklist_url.push("/licenses/".to_string());
    website.crawl();

    for page in website.get_pages() {
        println!("- {}", page.get_url());
    }
}
