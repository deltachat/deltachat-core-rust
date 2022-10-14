Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.mail.yahoo.com designates 77.238.177.32 as permitted sender)  smtp.mailfrom=alice@yahoo.com;
	dmarc=pass(p=reject dis=none)  header.from=yahoo.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.mail.yahoo.com designates 77.238.177.32 as permitted sender)  smtp.mailfrom=alice@yahoo.com;
	dmarc=pass header.from=<alice@yahoo.com> (p=reject dis=none)
From: <alice@yahoo.com>
To: <alice@zohomail.eu>
