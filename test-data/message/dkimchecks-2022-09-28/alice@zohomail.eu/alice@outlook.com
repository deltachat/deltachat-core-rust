Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of outlook.com designates 40.92.58.104 as permitted sender)  smtp.mailfrom=alice@outlook.com;
	arc=pass (i=1 dmarc=pass fromdomain=outlook.com);
	dmarc=pass(p=none dis=none)  header.from=outlook.com
ARC-Authentication-Results: i=2; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of outlook.com designates 40.92.58.104 as permitted sender)  smtp.mailfrom=alice@outlook.com;
	arc=pass (i=1 dmarc=pass fromdomain=outlook.com);
	dmarc=pass header.from=<alice@outlook.com> (p=none dis=none)
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@outlook.com>
To: <alice@zohomail.eu>
