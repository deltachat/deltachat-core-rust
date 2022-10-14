Authentication-Results: mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.mail.ru designates 94.100.181.251 as permitted sender)  smtp.mailfrom=alice@mail.ru;
	dmarc=pass(p=reject dis=none)  header.from=mail.ru
ARC-Authentication-Results: i=1; mx.zohomail.eu;
	dkim=pass;
	spf=pass (zohomail.eu: domain of _spf.mail.ru designates 94.100.181.251 as permitted sender)  smtp.mailfrom=alice@mail.ru;
	dmarc=pass header.from=<alice@mail.ru> (p=reject dis=none)
From: <alice@mail.ru>
To: <alice@zohomail.eu>
Authentication-Results: smtpng1.m.smailru.net; auth=pass smtp.auth=alice@mail.ru smtp.mailfrom=alice@mail.ru
