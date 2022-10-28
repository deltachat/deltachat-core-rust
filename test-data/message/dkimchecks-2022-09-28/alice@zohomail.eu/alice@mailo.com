Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of mailo.com designates 213.182.54.11 as permitted sender)  smtp.mailfrom=alice@mailo.com;
	dmarc=pass(p=none dis=none)  header.from=mailo.com
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of mailo.com designates 213.182.54.11 as permitted sender)  smtp.mailfrom=alice@mailo.com;
	dmarc=pass header.from=<alice@mailo.com> (p=none dis=none)
From: <alice@mailo.com>
To: <alice@zohomail.eu>
