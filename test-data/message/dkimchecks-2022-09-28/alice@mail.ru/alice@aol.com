From: <alice@aol.com>
To: <alice@mail.ru>
Authentication-Results: mxs.mail.ru; spf=pass (mx216.i.mail.ru: domain of aol.com designates 77.238.176.99 as permitted sender) smtp.mailfrom=alice@aol.com smtp.helo=sonic301-22.consmr.mail.ir2.yahoo.com;
	 dkim=pass header.d=aol.com; dmarc=pass header.from=alice@aol.com
