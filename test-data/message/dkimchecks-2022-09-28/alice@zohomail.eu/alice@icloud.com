Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of icloud.com designates 17.57.155.16 as permitted sender)  smtp.mailfrom=alice@icloud.com;
	dmarc=pass(p=quarantine dis=none)  header.from=icloud.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of icloud.com designates 17.57.155.16 as permitted sender)  smtp.mailfrom=alice@icloud.com;
	dmarc=pass header.from=<alice@icloud.com> (p=quarantine dis=none)
From: <alice@icloud.com>
To: <alice@zohomail.eu>
