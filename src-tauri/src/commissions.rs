//! Commission (외주) manager: art and design commissions moving through stages from the first
//! inquiry to delivery, with their price, payments, deadline and client.

use chrono::{Local, NaiveDate};
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Payment {
    /// In the currency chosen in the expense settings.
    pub amount: f64,
    pub date: NaiveDate,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commission {
    pub id: i64,
    pub title: String,
    pub client_id: Option<i64>,
    /// What kind of piece, e.g. "흉상", "SD", "로고"; free text.
    #[serde(default)]
    pub kind: String,
    /// One of `STATUSES`.
    pub status: String,
    #[serde(default)]
    pub price: f64,
    /// Money received so far (deposit, balance…), oldest first.
    #[serde(default)]
    pub payments: Vec<Payment>,
    #[serde(default)]
    pub deadline: Option<NaiveDate>,
    /// Revisions done so far.
    #[serde(default)]
    pub revisions: i64,
    #[serde(default)]
    pub links: Vec<String>,
    /// Finished piece or reference image, see `images.rs`.
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub created_at: i64,
    /// Set when the commission is moved to `done`, cleared when moved back.
    #[serde(default)]
    pub delivered: Option<NaiveDate>,
    /// Where it was taken (e.g. a commission site); empty for direct deals.
    #[serde(default)]
    pub platform: String,
    /// How the platform fee is given: "percent" of the price or a fixed "amount".
    #[serde(default = "Commission::default_fee_kind")]
    pub fee_kind: String,
    /// The platform fee (a percentage or an amount, see `fee_kind`).
    #[serde(default)]
    pub fee: f64,
}

impl Commission {
    pub const STATUSES: [&'static str; 5] = ["inquiry", "queued", "working", "review", "done"];
    pub const FEE_KINDS: [&'static str; 2] = ["percent", "amount"];

    fn default_fee_kind() -> String {
        "percent".into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommissionClient {
    pub id: i64,
    pub name: String,
    /// How to reach them (e.g. an X handle or e-mail); free text.
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub memo: String,
}

const COLUMNS: &str = "id, title, client_id, kind, status, price, payments, deadline, revisions, links, image, memo, created_at, \
     delivered, platform, fee_kind, fee";

/// Links are stored one per line, like event links.
fn links_text(links: &[String]) -> String {
    links.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

fn commission_from_row(r: &Row) -> DbResult<Commission> {
    let links: String = r.get(9)?;
    let payments: String = r.get(6)?;
    Ok(Commission {
        id: r.get(0)?,
        title: r.get(1)?,
        client_id: r.get(2)?,
        kind: r.get(3)?,
        status: r.get(4)?,
        price: r.get(5)?,
        payments: serde_json::from_str(&payments).unwrap_or_default(),
        deadline: r.get::<_, Option<String>>(7)?.map(parse_d),
        revisions: r.get(8)?,
        links: links.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect(),
        image: r.get(10)?,
        memo: r.get(11)?,
        created_at: r.get(12)?,
        delivered: r.get::<_, Option<String>>(13)?.map(parse_d),
        platform: r.get(14)?,
        fee_kind: r.get(15)?,
        fee: r.get(16)?,
    })
}

impl Db {
    pub fn commissions(&self) -> DbResult<Vec<Commission>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLUMNS} FROM commissions ORDER BY created_at DESC, id DESC"))?;
        st.query_map([], commission_from_row)?.collect()
    }

    pub fn save_commission(&self, c: &Commission) -> DbResult<i64> {
        // The delivery date follows the stage: set on reaching "done", cleared when moved back.
        let delivered = if c.status == "done" { Some(c.delivered.unwrap_or_else(|| Local::now().date_naive())) } else { None };
        let date = |d: Option<NaiveDate>| d.map(|d| d.format(D_FMT).to_string());
        let payments = serde_json::to_string(&c.payments).unwrap_or_else(|_| "[]".into());
        if c.id == 0 {
            let created = if c.created_at > 0 { c.created_at } else { now_ts() };
            self.conn.execute(
                "INSERT INTO commissions(title, client_id, kind, status, price, payments, deadline, revisions, links, image, memo,
                                         created_at, delivered, platform, fee_kind, fee)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    c.title, c.client_id, c.kind, c.status, c.price, payments, date(c.deadline), c.revisions,
                    links_text(&c.links), c.image, c.memo, created, date(delivered), c.platform, c.fee_kind, c.fee
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE commissions SET title=?1, client_id=?2, kind=?3, status=?4, price=?5, payments=?6, deadline=?7, revisions=?8,
                 links=?9, image=?10, memo=?11, delivered=?12, platform=?13, fee_kind=?14, fee=?15 WHERE id=?16",
                params![
                    c.title, c.client_id, c.kind, c.status, c.price, payments, date(c.deadline), c.revisions,
                    links_text(&c.links), c.image, c.memo, date(delivered), c.platform, c.fee_kind, c.fee, c.id
                ],
            )?;
            Ok(c.id)
        }
    }

    /// Deletes a commission and returns it, so the deletion can be undone.
    pub fn delete_commission(&self, id: i64) -> DbResult<Option<Commission>> {
        let stored = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM commissions WHERE id=?1"), [id], commission_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM commissions WHERE id=?1", [id])?;
        Ok(stored)
    }

    pub fn commission_clients(&self) -> DbResult<Vec<CommissionClient>> {
        let mut st = self.conn.prepare("SELECT id, name, contact, memo FROM commission_clients ORDER BY name COLLATE NOCASE")?;
        st.query_map([], |r| Ok(CommissionClient { id: r.get(0)?, name: r.get(1)?, contact: r.get(2)?, memo: r.get(3)? }))?
            .collect()
    }

    pub fn save_commission_client(&self, c: &CommissionClient) -> DbResult<i64> {
        if c.id == 0 {
            self.conn.execute(
                "INSERT INTO commission_clients(name, contact, memo) VALUES(?1, ?2, ?3)",
                params![c.name, c.contact, c.memo],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE commission_clients SET name=?1, contact=?2, memo=?3 WHERE id=?4",
                params![c.name, c.contact, c.memo, c.id],
            )?;
            Ok(c.id)
        }
    }

    /// Deletes a client; their commissions are kept without a client.
    pub fn delete_commission_client(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM commission_clients WHERE id=?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    fn commission(title: &str, status: &str) -> Commission {
        Commission {
            id: 0,
            title: title.into(),
            client_id: None,
            kind: "흉상".into(),
            status: status.into(),
            price: 50_000.0,
            payments: vec![],
            deadline: Some(d("2026-11-01")),
            revisions: 0,
            links: vec![],
            image: None,
            memo: String::new(),
            created_at: 0,
            delivered: None,
            platform: String::new(),
            fee_kind: "percent".into(),
            fee: 0.0,
        }
    }

    #[test]
    fn commissions_keep_payments_and_get_a_delivery_date_when_done() {
        let (db, path) = temp_db();
        let id = db.save_commission(&commission("Bust", "working")).unwrap();
        let mut c = db.commissions().unwrap().remove(0);
        assert_eq!(c.delivered, None);

        c.payments = vec![Payment { amount: 20_000.0, date: d("2026-10-01") }, Payment { amount: 30_000.0, date: d("2026-10-20") }];
        c.status = "done".into();
        db.save_commission(&c).unwrap();
        let done = db.commissions().unwrap().remove(0);
        assert_eq!(done.id, id);
        assert_eq!(done.payments.len(), 2);
        assert!(done.delivered.is_some());

        // Moved back to work: no longer delivered.
        db.save_commission(&Commission { status: "review".into(), platform: "크레페".into(), fee: 10.0, ..done }).unwrap();
        let fee = db.commissions().unwrap().remove(0);
        assert_eq!((fee.platform.as_str(), fee.fee_kind.as_str(), fee.fee), ("크레페", "percent", 10.0));
        assert_eq!(db.commissions().unwrap()[0].delivered, None);

        let removed = db.delete_commission(id).unwrap().unwrap();
        assert_eq!(removed.title, "Bust");
        assert!(db.commissions().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deleting_a_client_keeps_their_commissions() {
        let (db, path) = temp_db();
        let client = db
            .save_commission_client(&CommissionClient { id: 0, name: "노라".into(), contact: "@nora".into(), memo: String::new() })
            .unwrap();
        db.save_commission(&Commission { client_id: Some(client), ..commission("SD", "queued") }).unwrap();
        db.delete_commission_client(client).unwrap();
        let c = db.commissions().unwrap().remove(0);
        assert_eq!(c.client_id, None);
        assert!(db.commission_clients().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
