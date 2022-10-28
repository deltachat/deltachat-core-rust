Authentication-Results: bimi.icloud.com; bimi=none
Authentication-Results: dmarc.icloud.com; dmarc=pass header.from=yahoo.com
Authentication-Results: dkim-verifier.icloud.com;
	dkim=pass (2048-bit key) header.d=yahoo.com header.i=@yahoo.com header.b=ku0XoLqQ
Authentication-Results: spf.icloud.com; spf=pass (spf.icloud.com: domain of alice@yahoo.com designates 77.238.179.83 as permitted sender) smtp.mailfrom=alice@yahoo.com
From: <alice@yahoo.com>
To: <alice@icloud.com>
