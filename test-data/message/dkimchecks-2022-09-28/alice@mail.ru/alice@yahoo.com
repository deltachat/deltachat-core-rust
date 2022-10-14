From: <alice@yahoo.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx252.i.mail.ru: domain of yahoo.com designates 77.238.179.188 as permitted sender) smtp.mailfrom=alice@yahoo.com smtp.helo=sonic313-21.consmr.mail.ir2.yahoo.com;
	 dkim=pass header.d=yahoo.com; dmarc=pass header.from=alice@yahoo.com
