use std::{fmt::Debug, path::Path, sync::Arc};

use lettre::{
    message::SinglePart, transport::smtp::authentication::Credentials, Message, SmtpTransport,
    Transport,
};
use parking_lot::RwLock;

use crate::{
    email_limit::{EmailDomainAllowList, EmailDomainDenyList},
    file_watcher::FileWatcher,
};

pub trait EmailHook: Debug + Sync + Send {
    fn on_mail(&self, email_subject: &str, email_body: &str);
}

#[derive(Debug)]
struct EmailHookDummy {}
impl EmailHook for EmailHookDummy {
    fn on_mail(&self, _email_subject: &str, _email_body: &str) {
        // empty
    }
}

/// Shared email helper
#[derive(Debug)]
pub struct EmailShared {
    smtp: SmtpTransport,
    pub email_from: String,
    mail_hook: Arc<dyn EmailHook>,

    pub deny_list: RwLock<EmailDomainDenyList>,
    pub allow_list: RwLock<EmailDomainAllowList>,

    pub test_mode: bool,
}

impl EmailShared {
    pub async fn new(
        relay: &str,
        relay_port: u16,
        from_email: &str,
        username: &str,
        password: &str,
    ) -> anyhow::Result<Self> {
        let smtp = SmtpTransport::relay(relay)?
            .port(relay_port)
            .credentials(Credentials::new(username.into(), password.into()))
            .build();

        anyhow::ensure!(
            smtp.test_connection()?,
            "Could not connect to smtp server: {}",
            relay
        );
        Ok(Self {
            smtp,
            mail_hook: Arc::new(EmailHookDummy {}),
            email_from: from_email.into(),

            deny_list: RwLock::new(EmailDomainDenyList::load_from_file().await),
            allow_list: RwLock::new(EmailDomainAllowList::load_from_file().await),

            test_mode: false,
        })
    }

    /// A hook that can see all sent emails
    /// Currently only useful for testing
    #[allow(dead_code)]
    pub fn set_hook<F: EmailHook + 'static>(&mut self, hook: F) {
        self.mail_hook = Arc::new(hook);
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: String,
    ) -> anyhow::Result<()> {
        self.mail_hook.on_mail(subject, &html_body);
        let email = Message::builder()
            .from(self.email_from.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .singlepart(SinglePart::html(html_body))
            .unwrap();
        self.smtp.send(&email)?;

        Ok(())
    }

    const PATH: &str = "config/";
    pub async fn load_email_template(name: &str) -> anyhow::Result<String> {
        let path: &Path = Self::PATH.as_ref();
        Ok(tokio::fs::read_to_string(path.join(name)).await?)
    }

    pub fn watcher(name: &str) -> FileWatcher {
        FileWatcher::new(Self::PATH.as_ref(), name.as_ref())
    }

    #[cfg(test)]
    pub fn set_test_mode(&mut self, test_mode: bool) {
        self.test_mode = test_mode;
    }
    pub const fn test_mode(&self) -> bool {
        self.test_mode
    }
}

impl From<(&str, SmtpTransport)> for EmailShared {
    fn from((email_from, smtp): (&str, SmtpTransport)) -> Self {
        Self {
            smtp,
            mail_hook: Arc::new(EmailHookDummy {}),
            email_from: email_from.into(),

            deny_list: Default::default(),
            allow_list: Default::default(),

            test_mode: false,
        }
    }
}

#[cfg(test)]
mod test {
    use lettre::SmtpTransport;

    use crate::email::EmailShared;

    #[tokio::test]
    async fn email_test() {
        let email: EmailShared = ("test@localhost", SmtpTransport::unencrypted_localhost()).into();

        assert!(email.smtp.test_connection().unwrap());

        email
            .send_email(
                "TestTo <test@localhost>",
                "It works",
                "It indeed works".to_string(),
            )
            .await
            .unwrap();
    }
}
