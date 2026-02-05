use actix_web::{HttpResponse, error, http, web};
use chrono::{DateTime, FixedOffset};
use derive_more::{Display, Error, From};

pub const GOODREADS_USER: &str = "27549920";

#[derive(Debug, PartialEq, serde::Serialize)]
pub struct Book {
    pub title: String,
    pub authors: Vec<String>,
    pub url: String,
    pub date: DateTime<FixedOffset>,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[display("Could not parse XML ({})", source)]
    XmlParser {
        source: xee_xpath::error::DocumentsError,
    },

    #[display("XPath query failed: {} ({})", source.error.message(), source.error.code())]
    XpathQuery { source: xee_xpath::error::Error },

    #[display("HTTP request failed: {}", source)]
    HttpRequest { source: reqwest::Error },

    #[display("Could not parse URL: {}", source)]
    UrlParser { source: url::ParseError },
}

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(serde_json::json!({
          "error": self.to_string()
        }))
    }
    fn status_code(&self) -> http::StatusCode {
        match self {
            Self::HttpRequest { source } => source
                .status()
                .and_then(|code| code.as_u16().try_into().ok())
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
            _ => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn get_books(api_key: &redact::Secret<String>) -> Result<web::Json<Vec<Book>>, Error> {
    get_api_response(GOODREADS_USER, api_key)
        .await
        .and_then(|xml| parse_api_response(&xml))
        .map(web::Json)
}

struct Element<'a> {
    xot: &'a xot::Xot,
    node: xot::Node,
}

impl<'a> Element<'a> {
    pub fn from_node(xot: &'a xot::Xot, node: xot::Node) -> Self {
        Self { xot, node }
    }

    pub fn as_text(&self) -> Option<&'a str> {
        self.xot.text_content_str(self.node)
    }
}

impl Element<'_> {
    pub fn find_children(&self, name: &str) -> impl Iterator<Item = Element<'_>> {
        let name = self.xot.name(name);
        self.xot
            .children(self.node)
            .filter(|child| self.xot.is_element(*child))
            .filter(move |child| self.xot.node_name(*child) == name)
            .map(|node| Element {
                xot: self.xot,
                node,
            })
    }

    pub fn find_child(&self, name: &str) -> Option<Element<'_>> {
        self.find_children(name).next()
    }

    pub fn into_text(self) -> Option<String> {
        self.xot.text_content_str(self.node).map(Into::into)
    }
}

fn convert_review(review: Element<'_>) -> Option<Book> {
    let book = review.find_child("book")?;
    let title = book.find_child("title").and_then(Element::into_text)?;
    let authors = book
        .find_children("author")
        .flat_map(|a| a.find_child("name").and_then(Element::into_text))
        .collect();
    let url = book
        .find_child("id")
        .and_then(|e| e.as_text())
        .map(|id| format!("https://www.goodreads.com/book/show/{}", id))?;
    let date = review
        .find_child("created_at")
        .and_then(|e| e.as_text())
        .map(DateTime::<FixedOffset>::parse_from_rfc3339)
        .and_then(Result::ok)?;

    Some(Book {
        title,
        authors,
        url,
        date,
    })
}

fn parse_api_response(xml: &str) -> Result<Vec<Book>, Error> {
    use xee_xpath::Query;
    let mut docs = xee_xpath::Documents::new();
    let queries = xee_xpath::Queries::default();
    let doc = docs.add_string_without_uri(xml)?;
    queries
        .many(
            r"//update[@type='readstatus']/object/read_status[status='currently-reading']/review",
            |doc, item| {
                item.to_node()
                    .map_err(Into::into)
                    .map(|n| Element::from_node(doc.xot(), n))
                    .map(convert_review)
            },
        )
        .and_then(|q| q.execute(&mut docs, doc))
        .map_err(Into::into)
        .map(|books| books.into_iter().flatten().collect())
}

async fn get_api_response(
    user_id: &str,
    api_key: &redact::Secret<String>,
) -> Result<String, Error> {
    use futures::TryFutureExt;
    reqwest::get(reqwest::Url::parse_with_params(
        &format!("https://www.goodreads.com/user/show/{}.xml", user_id),
        &[("key", api_key.expose_secret())],
    )?)
    .and_then(|rsp| rsp.text())
    .map_err(|e| e.without_url().into())
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_web::test]
    async fn make_api_request() -> Result<(), Error> {
        let _ = dotenvy::dotenv();
        assert!(
            get_api_response(
                GOODREADS_USER,
                &dotenvy::var("GOODREADS_API_KEY")
                    .expect("Missing API key")
                    .into()
            )
            .await
            .is_ok()
        );
        Ok(())
    }

    #[test]
    fn parse_test_data() -> Result<(), Error> {
        use chrono::prelude::*;
        let actual: Vec<Book> = parse_api_response(TEST_DATA)?;
        let expected = Book {
            title: "The Price of Peace: Money, Democracy, and the Life of John Maynard Keynes"
                .into(),
            authors: vec!["Zachary D. Carter".into()],
            url: "https://www.goodreads.com/book/show/49644992".into(),
            date: Utc
                .with_ymd_and_hms(2026, 1, 24, 11, 5, 48)
                .single()
                .map(Into::into)
                .unwrap(),
        };

        assert_eq!(actual, vec![expected]);
        Ok(())
    }

    const TEST_DATA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<GoodreadsResponse>
  <Request>
    <authentication>true</authentication>
    <key><![CDATA[ry3enqxrtVbFpWRCEUXfbg]]></key>
    <method><![CDATA[user_show]]></method>
  </Request>
  <user>
    <id>27549920</id>
    <name>Simon Sigurdhsson</name>
    <user_name>urdh</user_name>
    <link><![CDATA[https://www.goodreads.com/user/show/27549920-simon-sigurdhsson]]></link>
    <image_url><![CDATA[https://images.gr-assets.com/users/1412036492p3/27549920.jpg]]></image_url>
    <small_image_url><![CDATA[https://images.gr-assets.com/users/1412036492p2/27549920.jpg]]></small_image_url>
    <about></about>
    <age>35</age>
    <gender></gender>
    <location></location>
    <website>http://sigurdhsson.org</website>
    <joined>01/2014</joined>
    <last_active>02/2026</last_active>
    <interests></interests>
    <favorite_books></favorite_books>
    <favorite_authors />
    <updates_rss_url><![CDATA[https://www.goodreads.com/user/updates_rss/27549920?key=CTDhL4twz2Bimkc2QISHJdS0AJBJ5kv9Wg_1rNd8EsSZxx5F]]></updates_rss_url>
    <reviews_rss_url><![CDATA[https://www.goodreads.com/review/list_rss/27549920?key=CTDhL4twz2Bimkc2QISHJdS0AJBJ5kv9Wg_1rNd8EsSZxx5F&shelf=%23ALL%23]]></reviews_rss_url>
    <friends_count type="integer">13</friends_count>
    <groups_count>0</groups_count>
    <reviews_count type="integer">120</reviews_count>
    <user_shelves type="array">
      <user_shelf>
        <id type="integer">89273606</id>
        <name>to-read</name>
        <book_count type="integer">66</book_count>
        <exclusive_flag type="boolean">true</exclusive_flag>
        <sort nil="true" />
        <order>a</order>
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">89273607</id>
        <name>currently-reading</name>
        <book_count type="integer">1</book_count>
        <exclusive_flag type="boolean">true</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">89273608</id>
        <name>read</name>
        <book_count type="integer">52</book_count>
        <exclusive_flag type="boolean">true</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">true</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">115785701</id>
        <name>abandoned</name>
        <book_count type="integer">1</book_count>
        <exclusive_flag type="boolean">true</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">124753994</id>
        <name>on-hold</name>
        <book_count type="integer">3</book_count>
        <exclusive_flag type="boolean">true</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114817461</id>
        <name>comedy</name>
        <book_count type="integer">2</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">169397296</id>
        <name>economics</name>
        <book_count type="integer">11</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114817332</id>
        <name>environment</name>
        <book_count type="integer">1</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114817183</id>
        <name>fiction</name>
        <book_count type="integer">15</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114817327</id>
        <name>history</name>
        <book_count type="integer">9</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114816960</id>
        <name>philosophy</name>
        <book_count type="integer">9</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">189895517</id>
        <name>photography</name>
        <book_count type="integer">10</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114816944</id>
        <name>politics</name>
        <book_count type="integer">36</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114817624</id>
        <name>technology</name>
        <book_count type="integer">17</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">95373721</id>
        <name>to-buy</name>
        <book_count type="integer">31</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">114815875</id>
        <name>to-loan</name>
        <book_count type="integer">3</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">false</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
      <user_shelf>
        <id type="integer">177049900</id>
        <name>wishlist</name>
        <book_count type="integer">18</book_count>
        <exclusive_flag type="boolean">false</exclusive_flag>
        <sort nil="true" />
        <order nil="true" />
        <per_page type="integer" nil="true" />
        <display_fields></display_fields>
        <featured type="boolean">false</featured>
        <recommend_for type="boolean">true</recommend_for>
        <sticky type="boolean" nil="true" />
      </user_shelf>
    </user_shelves>
    <updates type="array">
      <update type="readstatus">
        <action_text><![CDATA[is currently reading <a only_path="false" href="https://www.goodreads.com/review/show/8294180710">The Price of Peace: Money, Democracy, and the Life of John Maynard Keynes</a>]]></action_text>
        <link>https://www.goodreads.com/read_statuses/10449998545</link>
        <image_url>https://images.gr-assets.com/users/1412036492p2/27549920.jpg</image_url>
        <actor>
          <id type="integer">27549920</id>
          <name>Simon Sigurdhsson</name>
          <image_url>https://images.gr-assets.com/users/1412036492p2/27549920.jpg</image_url>
          <link>https://www.goodreads.com/user/show/27549920-simon-sigurdhsson</link>
        </actor>
        <updated_at>Sat, 24 Jan 2026 03:05:48 -0800</updated_at>
        <object>
          <read_status>
            <id type="integer">10449998545</id>
            <review_id type="integer">8294180710</review_id>
            <user_id type="integer">27549920</user_id>
            <old_status nil="true" />
            <status>currently-reading</status>
            <updated_at type="dateTime">2026-01-24T11:05:48+00:00</updated_at>
            <review>
              <id type="integer">8294180710</id>
              <user_id type="integer">27549920</user_id>
              <book_id type="integer">49644992</book_id>
              <rating type="integer">0</rating>
              <read_status>currently-reading</read_status>
              <sell_flag type="boolean">false</sell_flag>
              <review nil="true" />
              <updated_at type="dateTime">2026-01-24T11:05:48+00:00</updated_at>
              <created_at type="dateTime">2026-01-24T11:05:48+00:00</created_at>
              <comments_count type="integer">0</comments_count>
              <weight type="integer">0</weight>
              <ratings_sum type="integer">0</ratings_sum>
              <ratings_count type="integer">0</ratings_count>
              <spoiler_flag type="boolean">false</spoiler_flag>
              <work_id type="integer">73187380</work_id>
              <last_comment_at type="dateTime" nil="true" />
              <hidden_flag type="boolean">false</hidden_flag>
              <language_code type="integer" nil="true" />
              <last_revision_at type="dateTime">2026-01-24T11:05:48+00:00</last_revision_at>
              <non_friends_rating_count type="integer">0</non_friends_rating_count>
              <encrypted_notes nil="true" />
              <book_uri>kca://book/amzn1.gr.book.v3.JKlLGH7niiJBgGL0</book_uri>
              <notes nil="true" />
              <book>
                <id type="integer">49644992</id>
                <work_id type="integer">73187380</work_id>
                <isbn>0525509038</isbn>
                <isbn13>9780525509035</isbn13>
                <title>The Price of Peace: Money, Democracy, and the Life of John Maynard Keynes</title>
                <sort_by_title>price of peace: money, democracy, and the life of john maynard
                  keynes, the</sort_by_title>
                <author_id type="integer">19520407</author_id>
                <author_role nil="true" />
                <asin>0525509038</asin>
                <description>&lt;b&gt;A page-turning biography of world-changing economist John
                  Maynard Keynes and the big ideas that outlived him.&lt;/b&gt;

                  In the spring of 1934, Virginia Woolf sketched an affectionate biographical
                  portrait of her great friend John Maynard Keynes. Writing a full two years before
                  Keynes would revolutionize the economics world with the publication of
                  &lt;i&gt;The General Theory&lt;/i&gt;, Woolf nevertheless found herself unable to
                  condense her friend's already-extraordinary life into anything less than
                  twenty-five themes, which she jotted down at the opening of her homage: "Politics.
                  Art. Dancing. Letters. Economics. Youth. The Future. Glands. Genealogies.
                  Atlantis. Mortality. Religion. Cambridge. Eton. The Drama. Society. Truth. Pigs.
                  Sussex. The History of England. America. Optimism. Stammer. Old Books. Hume."
                  Keynes was not only an economist, as he is remembered today, but the preeminent
                  anti-authoritarian thinker of the twentieth century, a man who devoted his life to
                  the belief that art and ideas could conquer war and deprivation. A moral
                  philosopher, political theorist, and statesman, Keynes immersed himself in a
                  creative milieu filled with ballerinas and literary icons as he developed his own
                  innovative and at times radical thought, reinventing Enlightenment liberalism for
                  the harrowing crises of his day--which included two world wars and an economic
                  collapse that challenged the legitimacy of democratic government itself.
                  &lt;i&gt;The Price of Peace&lt;/i&gt; follows Keynes from intimate
                  turn-of-the-century parties in London's riotous Bloomsbury art scene to the
                  fevered negotiations in Paris that shaped the Treaty of Versailles, through stock
                  market crashes and currency crises to diplomatic breakthroughs in the mountains of
                  New Hampshire and wartime ballet openings at Covent Garden.

                  In this riveting biography, veteran journalist Zachary D. Carter unearths the lost
                  legacy of one of history's most important minds. John Maynard Keynes's vibrant,
                  deeply human vision of democracy, art, and the good life has been obscured by
                  technical debates, but in &lt;i&gt;The Price of Peace&lt;/i&gt;, Carter revives a
                  forgotten set of ideas with the power to reinvent national government and reframe
                  the principles of international diplomacy in our own time.</description>
                <format>Hardcover</format>
                <publication_year type="integer">2020</publication_year>
                <publication_month type="integer">5</publication_month>
                <publication_day type="integer">19</publication_day>
                <num_pages type="integer">656</num_pages>
                <publisher>Random House</publisher>
                <language_code>eng</language_code>
                <edition_information>First Edition</edition_information>
                <url>https://www.penguinrandomhouse.com/books/563378/the-price-of-peace-by-zachary-d-carter/</url>
                <source_url nil="true" />
                <image_uploaded_at type="dateTime">2024-08-09T02:23:26+00:00</image_uploaded_at>
                <s3_image_at type="dateTime">2024-08-09T02:23:27+00:00</s3_image_at>
                <reviews_count type="integer">16342</reviews_count>
                <ratings_sum type="integer">12476</ratings_sum>
                <ratings_count type="integer">2833</ratings_count>
                <text_reviews_count type="integer">448</text_reviews_count>
                <book_authors_count type="integer">0</book_authors_count>
                <updated_at type="dateTime">2026-02-05T16:56:44+00:00</updated_at>
                <created_at type="dateTime">2019-12-21T23:50:03+00:00</created_at>
                <author_id_updater_user_id type="integer">-1001</author_id_updater_user_id>
                <author_role_updater_user_id type="integer">-12</author_role_updater_user_id>
                <description_updater_user_id type="integer">6678151</description_updater_user_id>
                <edition_information_updater_user_id type="integer">-12</edition_information_updater_user_id>
                <format_updater_user_id type="integer">-1001</format_updater_user_id>
                <image_updater_user_id type="integer">116922418</image_updater_user_id>
                <isbn_updater_user_id type="integer">-1001</isbn_updater_user_id>
                <isbn13_updater_user_id type="integer">-1001</isbn13_updater_user_id>
                <language_updater_user_id type="integer">6678151</language_updater_user_id>
                <num_pages_updater_user_id type="integer">12233925</num_pages_updater_user_id>
                <publication_date_updater_user_id type="integer">-1001</publication_date_updater_user_id>
                <publisher_updater_user_id type="integer">-1001</publisher_updater_user_id>
                <title_updater_user_id type="integer">-1001</title_updater_user_id>
                <url_updater_user_id type="integer">6678151</url_updater_user_id>
                <asin_updater_user_id type="integer">0</asin_updater_user_id>
                <book_uri>kca://book/amzn1.gr.book.v3.JKlLGH7niiJBgGL0</book_uri>
                <classification>Public</classification>
                <classification_updater_user_id type="integer" nil="true" />
                <classification_updated_at type="dateTime" nil="true" />
                <created_by_user_id type="integer" nil="true" />
                <author>
                  <id type="integer">19520407</id>
                  <name>Zachary D. Carter</name>
                  <updated_at type="dateTime">2026-02-04T16:56:28+00:00</updated_at>
                  <created_at type="dateTime">2019-09-02T16:36:09+00:00</created_at>
                  <image_uploaded_at type="dateTime" nil="true" />
                  <user_id type="integer" nil="true" />
                  <country_code></country_code>
                  <born_at type="dateTime" nil="true" />
                  <died_at type="dateTime" nil="true" />
                  <about></about>
                  <uploader_user_id type="integer">0</uploader_user_id>
                  <image_copyright></image_copyright>
                  <influences></influences>
                  <url>https://zacharydcarter.substack.com/</url>
                  <genre1></genre1>
                  <genre2></genre2>
                  <genre3></genre3>
                  <books_count type="integer">15</books_count>
                  <reviews_count type="integer">18511</reviews_count>
                  <ratings_sum type="integer">15329</ratings_sum>
                  <works_count type="integer">1</works_count>
                  <hometown></hometown>
                  <rating_dist nil="true" />
                  <s3_image_at type="dateTime" nil="true" />
                  <ratings_count type="integer">3461</ratings_count>
                  <text_reviews_count type="integer">502</text_reviews_count>
                  <author_program_at type="dateTime" nil="true" />
                  <best_book_id type="integer">49644992</best_book_id>
                  <sort_by_name>carter, zachary d.</sort_by_name>
                  <shelf_display_name>Carter, Zachary D.</shelf_display_name>
                  <author_uri>kca://author/amzn1.gr.author.v2.5c4acb87-8852-42d8-b1bb-cb8996e0628c</author_uri>
                  <name_lower>zachary d. carter</name_lower>
                  <asin>B07XZY9F4Y</asin>
                </author>
              </book>
            </review>
          </read_status>
        </object>
      </update>
    </updates>
    <user_statuses>
    </user_statuses>
  </user>
</GoodreadsResponse>
"#;
}
