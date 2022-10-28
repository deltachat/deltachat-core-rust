Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.google.com designates 209.85.221.68 as permitted sender)  smtp.mailfrom=alice@gmail.com;
	dmarc=pass(p=none dis=none)  header.from=gmail.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.google.com designates 209.85.221.68 as permitted sender)  smtp.mailfrom=alice@gmail.com;
	dmarc=pass header.from=<alice@gmail.com> (p=none dis=none)
From: <alice@gmail.com>
To: <alice@zohomail.eu>
