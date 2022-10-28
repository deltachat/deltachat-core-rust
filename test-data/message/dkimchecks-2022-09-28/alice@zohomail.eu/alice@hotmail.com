Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of hotmail.com designates 40.92.89.94 as permitted sender)  smtp.mailfrom=alice@hotmail.com;
	arc=pass (i=1 dmarc=pass fromdomain=hotmail.com);
	dmarc=pass(p=none dis=none)  header.from=hotmail.com
ARC-Authentication-Results: i=2; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of hotmail.com designates 40.92.89.94 as permitted sender)  smtp.mailfrom=alice@hotmail.com;
	arc=pass (i=1 dmarc=pass fromdomain=hotmail.com);
	dmarc=pass header.from=<alice@hotmail.com> (p=none dis=none)
ARC-Authentication-Results: i=1; mx.microsoft.com 1; spf=none; dmarc=none;
 dkim=none; arc=none
From: <alice@hotmail.com>
To: <alice@zohomail.eu>
