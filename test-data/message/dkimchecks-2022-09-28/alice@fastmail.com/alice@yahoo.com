ARC-Authentication-Results: i=1; mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=sonic310-11.consmr.mail.ir2.yahoo.com
    policy.ptr=sonic310-11.consmr.mail.ir2.yahoo.com;
    bimi=none (No BIMI records found);
    arc=none (no signatures found);
    dkim=pass (2048-bit rsa key sha256) header.d=yahoo.com
    header.i=@yahoo.com header.b=cynWU+nU header.a=rsa-sha256
    header.s=s2048 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
    policy.applied-disposition=none policy.evaluated-disposition=none
    (p=reject,d=none,d.eval=none) policy.policy-from=p
    header.from=yahoo.com;
    iprev=pass smtp.remote-ip=77.238.177.32
    (sonic310-11.consmr.mail.ir2.yahoo.com);
    spf=pass smtp.mailfrom=alice@yahoo.com
    smtp.helo=sonic310-11.consmr.mail.ir2.yahoo.com
Authentication-Results: mx1.messagingengine.com;
    x-csa=none;
    x-me-sender=none;
    x-ptr=pass smtp.helo=sonic310-11.consmr.mail.ir2.yahoo.com
      policy.ptr=sonic310-11.consmr.mail.ir2.yahoo.com
Authentication-Results: mx1.messagingengine.com;
    bimi=none (No BIMI records found)
Authentication-Results: mx1.messagingengine.com;
    arc=none (no signatures found)
Authentication-Results: mx1.messagingengine.com;
    dkim=pass (2048-bit rsa key sha256) header.d=yahoo.com
      header.i=@yahoo.com header.b=cynWU+nU header.a=rsa-sha256
      header.s=s2048 x-bits=2048;
    dmarc=pass policy.published-domain-policy=reject
      policy.applied-disposition=none policy.evaluated-disposition=none
      (p=reject,d=none,d.eval=none) policy.policy-from=p
      header.from=yahoo.com;
    iprev=pass smtp.remote-ip=77.238.177.32
      (sonic310-11.consmr.mail.ir2.yahoo.com);
    spf=pass smtp.mailfrom=alice@yahoo.com
      smtp.helo=sonic310-11.consmr.mail.ir2.yahoo.com
From: <alice@yahoo.com>
To: <alice@fastmail.com>
