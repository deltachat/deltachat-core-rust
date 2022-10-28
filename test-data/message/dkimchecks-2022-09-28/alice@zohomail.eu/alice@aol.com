Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of aol.com designates 77.238.177.146 as permitted sender)  smtp.mailfrom=alice@aol.com;
	dmarc=pass(p=reject dis=none)  header.from=aol.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of aol.com designates 77.238.177.146 as permitted sender)  smtp.mailfrom=alice@aol.com;
	dmarc=pass header.from=<alice@aol.com> (p=reject dis=none)
From: <alice@aol.com>
To: <alice@zohomail.eu>
