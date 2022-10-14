Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of fastmail.com designates 66.111.4.28 as permitted sender)  smtp.mailfrom=alice@fastmail.com;
	dmarc=pass(p=none dis=none)  header.from=fastmail.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of fastmail.com designates 66.111.4.28 as permitted sender)  smtp.mailfrom=alice@fastmail.com;
	dmarc=pass header.from=<alice@fastmail.com> (p=none dis=none)
From: <alice@fastmail.com>
To: <alice@zohomail.eu>
